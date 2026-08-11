//! Patch hardcoded EN texts that bypass the FFF8 pointer table.
//!
//! Two categories of hardcoded text exist in the EN patch:
//!
//! 1. **Cait Sith plural-form**: Special 68K handlers for plural monsters
//!    that use inline text data via LEA (4 texts).
//!
//! 2. **Momomo Fire Extinguisher**: The item name "Fire Extinguisher" is too
//!    long for Momomo's deposit/withdraw dialog, so the EN patch hardcodes
//!    special-case text when item ID == 0x24 (2 texts).
//!
//! Source: madou1mdtools/madou1md/script/new.txt
//!
//! Strategy: Read KR text from translations, encode, write to ROM after main
//! text area, and patch LEA addresses in handlers.

use std::collections::HashMap;

use super::text;

/// A hardcoded text patch definition.
struct PluralPatch {
    /// Translation key (e.g. "plural_capsule_prompt")
    key: &'static str,
    /// EN text (for fallback / reference)
    en_text: &'static str,
    /// ROM addresses of LEA operand (the 4-byte address after LEA opcode).
    /// We patch [addr..addr+4] with the new data address.
    lea_addrs: &'static [usize],
}

const PATCHES: &[PluralPatch] = &[
    // --- Cait Sith plural-form texts ---
    PluralPatch {
        key: "plural_capsule_prompt",
        // JP: [name]がカプセルの中でねむってるよ。逃がしてあげる？
        en_text: "{FF10}The {FF78:FF30}are sleeping{NL}inside the{NL}Capsule.{PAGE}Release the{NL}monsters,{NL}{pad}{pad}▸Yes{NL}{pad}{pad}▸No{FF74}{FFFF}",
        lea_addrs: &[
            0x300694 + 30, // capsule_prompt handler: LEA $XXXXXXXX, a2
        ],
    },
    PluralPatch {
        key: "plural_capsule_released",
        // JP: [name]は元気よくそとに出た。「かえっていいよ」[name]はさみしそうにさっていった。
        en_text: "{FF10}The {FF78:FF30}leave the{NL}Capsule! eager to{NL}please.{PAGE}{FF10}{q-open}You can go{NL}now.{q}{PAGE}{FF10}The {FF78:FF30}look sad! but{NL}comply.{PAGE}{FFFF}",
        lea_addrs: &[
            0x3006C2 + 30, // capsule_released handler
            0x300694 + 76, // also referenced from capsule_prompt handler
        ],
    },
    PluralPatch {
        key: "plural_monster_encounter",
        // JP: まものだ！
        en_text: "{FF10}Monsters!{END}",
        lea_addrs: &[
            0x3006F0 + 30, // monster_encounter handler
            0x3006C2 + 76, // also referenced from capsule_released handler
        ],
    },
    PluralPatch {
        key: "plural_amigo_defeated",
        // JP: [name]はばたんきゅー。
        en_text: "{FF10}The {FF78:FF30}are defeated.{FF38}",
        lea_addrs: &[
            0x30071A + 38, // amigo_defeat handler
            0x3006F0 + 80, // also referenced from monster_encounter handler
        ],
    },
    // --- Momomo Fire Extinguisher special-case texts ---
    // EN: item name "Fire Extinguisher" too long for deposit/withdraw dialog.
    // 68K code at 0x3008EE checks item ID == 0x24, branches to LEA with inline text.
    PluralPatch {
        key: "momomo_fireext_deposit",
        // JP: 「[item]あずかるのー」 (Momomo deposit)
        en_text: "{FF64}{FF14}{q-open}Fire Extingui-{NL}sher, okaaay.{q}{PAGE}{END}",
        lea_addrs: &[
            0x3008FA, // newMomomoDeposit: LEA momomoDeposit_fireExtinguisher, a2
        ],
    },
    PluralPatch {
        key: "momomo_fireext_withdraw",
        // JP: 「[item]わたすー」 (Momomo withdraw)
        en_text: "{FF64}{FF14}{q-open}Fire Extingui-{NL}sher, heeere you{NL}go.{q}{PAGE}{END}",
        lea_addrs: &[
            0x300928, // newMomomoWithdraw: LEA momomoWithdraw_fireExtinguisher, a2
        ],
    },
];

/// Encode and patch all Cait Sith plural texts.
/// `text_offset` is the current write position in ROM (after main text data).
/// Returns the new text_offset after writing.
pub fn patch_plural_texts(
    rom: &mut [u8],
    mut text_offset: usize,
    kr_charmap: &HashMap<char, u16>,
    en_charmap: &HashMap<char, u16>,
    translations: &HashMap<String, String>,
) -> Result<usize, String> {
    let mut count = 0;

    for patch in PATCHES {
        // Look up KR translation; fall back to EN text
        let text_src = translations
            .get(patch.key)
            .map(|s| s.as_str())
            .unwrap_or(patch.en_text);

        let is_kr = translations.contains_key(patch.key);
        let label = if is_kr { "KR" } else { "EN fallback" };

        let words = text::encode_text(text_src, kr_charmap, en_charmap)
            .map_err(|e| format!("plural '{}' encode failed: {e}", patch.key))?;
        let encoded = text::words_to_bytes(&words);

        if text_offset + encoded.len() > rom.len() {
            return Err(format!(
                "plural '{}': ROM overflow at 0x{:06X}",
                patch.key, text_offset
            ));
        }
        rom[text_offset..text_offset + encoded.len()].copy_from_slice(&encoded);

        // Patch all LEA address operands
        let addr_bytes = (text_offset as u32).to_be_bytes();
        for &lea_addr in patch.lea_addrs {
            // Verify LEA opcode (45F9=a2 or 47F9=a3) is 2 bytes before
            let op = u16::from_be_bytes([rom[lea_addr - 2], rom[lea_addr - 1]]);
            if op != 0x45F9 && op != 0x47F9 {
                return Err(format!(
                    "plural '{}': expected LEA opcode at 0x{:06X}, found 0x{:04X}",
                    patch.key,
                    lea_addr - 2,
                    op
                ));
            }
            rom[lea_addr..lea_addr + 4].copy_from_slice(&addr_bytes);
        }

        eprintln!(
            "  {}: {} bytes at 0x{:06X} [{}] ({} refs)",
            patch.key,
            encoded.len(),
            text_offset,
            label,
            patch.lea_addrs.len()
        );
        text_offset += encoded.len();
        count += 1;
    }

    eprintln!("  {} plural texts patched", count);
    Ok(text_offset)
}
