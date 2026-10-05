//! M12 诊断：本机 CPAL 最小回调探针（绕开全部引擎逻辑）。
//! 运行：cargo run -p aura-core --example cpal_probe
//! 预期：2 秒内 callback 计数 > 0；若 error 计数 > 0 或 callback=0，
//! 说明环境/设备层问题而非 ifuyo 引擎问题。

fn main() {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    let host = cpal::default_host();
    println!("host: {:?}", host.id());
    let device = host
        .default_output_device()
        .expect("no default output device");
    let supported = device.default_output_config().expect("no config");
    println!("default config: {supported:?}");

    let callbacks = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let cb2 = Arc::clone(&callbacks);
    let err2 = Arc::clone(&errors);

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                supported.config(),
                move |_data: &mut [f32], _| {
                    cb2.fetch_add(1, Ordering::Relaxed);
                },
                move |err| {
                    err2.fetch_add(1, Ordering::Relaxed);
                    eprintln!("stream err: {err}");
                },
                None,
            )
            .expect("build stream"),
        other => {
            println!("unsupported sample format: {other:?}");
            return;
        }
    };
    stream.play().expect("stream.play()");

    std::thread::sleep(std::time::Duration::from_secs(2));
    println!(
        "RESULT: callbacks={} errors={} (2s)",
        callbacks.load(Ordering::Relaxed),
        errors.load(Ordering::Relaxed)
    );
}
