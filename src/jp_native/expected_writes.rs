//! Fail-closed ROM mutation plans and post-write verification.

use super::{CHECKSUM_START, JP_EXPANDED_ROM_SIZE, JP_ROM_SIZE};

pub(super) struct ExpectedWrite {
    pub(super) label: String,
    pub(super) offset: usize,
    pub(super) expected: Vec<u8>,
    pub(super) replacement: Vec<u8>,
}

pub(super) fn expand_with_ff(source: &[u8]) -> Result<Vec<u8>, String> {
    if source.len() != JP_ROM_SIZE {
        return Err(format!(
            "JP expansion source size mismatch: expected 0x{JP_ROM_SIZE:X}, got 0x{:X}",
            source.len()
        ));
    }
    let mut output = Vec::with_capacity(JP_EXPANDED_ROM_SIZE);
    output.extend_from_slice(source);
    output.resize(JP_EXPANDED_ROM_SIZE, 0xFF);
    Ok(output)
}

pub(super) fn validate_raw_expansion(
    source: &[u8],
    output: &[u8],
    writes: &[ExpectedWrite],
) -> Result<(), String> {
    if output.len() != JP_EXPANDED_ROM_SIZE {
        return Err(format!(
            "expanded ROM size mismatch: expected 0x{JP_EXPANDED_ROM_SIZE:X}, got 0x{:X}",
            output.len()
        ));
    }
    if output[JP_ROM_SIZE..].iter().any(|&byte| byte != 0xFF) {
        return Err("raw expansion tail contains non-0xFF data".to_string());
    }
    for (offset, (&before, &after)) in source.iter().zip(output).enumerate() {
        if before != after
            && !writes.iter().any(|write| {
                (write.offset..write.offset + write.replacement.len()).contains(&offset)
            })
        {
            return Err(format!(
                "raw expansion changed unplanned source byte at 0x{offset:06X}"
            ));
        }
    }
    Ok(())
}

pub(super) fn calculate_checksum(rom: &[u8]) -> u16 {
    rom[CHECKSUM_START..]
        .chunks_exact(2)
        .fold(0u16, |checksum, pair| {
            checksum.wrapping_add(u16::from_be_bytes([pair[0], pair[1]]))
        })
}

pub(super) fn expect_bytes(
    data: &[u8],
    offset: usize,
    expected: &[u8],
    label: &str,
) -> Result<(), String> {
    let actual = data
        .get(offset..offset + expected.len())
        .ok_or_else(|| format!("{label}: range is outside the ROM"))?;
    if actual != expected {
        return Err(format!(
            "{label}: expected bytes do not match at 0x{offset:06X}"
        ));
    }
    Ok(())
}

pub(super) fn validate_plan(source: &[u8], writes: &[ExpectedWrite]) -> Result<(), String> {
    let mut ranges: Vec<(usize, usize, &str)> = Vec::with_capacity(writes.len());
    for write in writes {
        if write.expected.len() != write.replacement.len() {
            return Err(format!(
                "{}: expected/replacement lengths differ",
                write.label
            ));
        }
        expect_bytes(source, write.offset, &write.expected, &write.label)?;
        ranges.push((
            write.offset,
            write.offset + write.replacement.len(),
            &write.label,
        ));
    }
    ranges.sort_unstable_by_key(|range| range.0);
    for pair in ranges.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(format!(
                "Expected Writes overlap: {} and {}",
                pair[0].2, pair[1].2
            ));
        }
    }
    Ok(())
}

pub(super) fn apply_plan(rom: &mut [u8], writes: &[ExpectedWrite]) {
    for write in writes {
        rom[write.offset..write.offset + write.replacement.len()]
            .copy_from_slice(&write.replacement);
    }
}

pub(super) fn validate_result(
    source: &[u8],
    output: &[u8],
    writes: &[ExpectedWrite],
) -> Result<(), String> {
    for write in writes {
        let actual = &output[write.offset..write.offset + write.replacement.len()];
        if actual != write.replacement {
            return Err(format!("{}: replacement verification failed", write.label));
        }
    }

    for (offset, (before, after)) in source.iter().zip(output).enumerate() {
        if before != after
            && !writes.iter().any(|write| {
                (write.offset..write.offset + write.replacement.len()).contains(&offset)
            })
        {
            return Err(format!("unplanned ROM difference at 0x{offset:06X}"));
        }
    }
    Ok(())
}
