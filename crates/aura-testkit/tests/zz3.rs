use aura_testkit::synth_flac::encode_flac_mono_16bit;

#[test]
fn zz3_crc16_check() {
    let pcm = vec![100i16; 4096];
    let flac = encode_flac_mono_16bit(&pcm, 48_000);
    // frame 0: bytes 42..2103 (CRC-16 = 最后两字节)
    let frame = &flac[42..2103];
    let stored = u16::from_be_bytes([frame[frame.len() - 2], frame[frame.len() - 1]]);
    println!("FRAME0 len={} stored_crc16={:04x}", frame.len(), stored);
    // 标准 CRC-16 (poly 0x8005, init 0, MSB-first)
    let mut crc: u16 = 0;
    for &b in &frame[..frame.len() - 2] {
        crc ^= u16::from(b) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
        }
    }
    println!("computed crc16={crc:04x}");
}
