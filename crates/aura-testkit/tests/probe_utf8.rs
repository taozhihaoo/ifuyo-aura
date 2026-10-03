use aura_testkit::synth_flac::encode_flac_mono_16bit;

#[test]
fn probe_utf8_and_header() {
    // encode_utf8_frame_number 是私有函数；通过公开编码器间接验证：
    // 帧号 0,1,2,3 的 UTF-8 应为单字节 00,01,02,03
    let pcm = vec![100i16; 4096]; // 4 帧
    let flac = encode_flac_mono_16bit(&pcm, 48_000);
    println!("f0 hdr: {:02x?}", &flac[42..50]);
    println!("f1 hdr: {:02x?}", &flac[2103..2111]);
    println!("f2 hdr: {:02x?}", &flac[4164..4172]);
    println!("f3 hdr: {:02x?}", &flac[6225..6233]);
    panic!("dump");
}
