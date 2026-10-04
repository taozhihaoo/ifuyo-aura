//! 内部 radix-2 FFT（§80）：小型、自实现、经测试；不为一个 FFT 引入
//! 大型 DSP framework。f64 内部精度（避免大规模累加误差），输出 f32。

/// 前向 FFT（Cooley–Tukey，迭代 radix-2）。`input.len()` 必须为 2 的幂。
///
/// # Errors
/// 长度非 2 的幂 → `&'static str`。
pub fn fft_forward(input: &[f32]) -> Result<(Vec<f32>, Vec<f32>), &'static str> {
    let n = input.len();
    if n == 0 || !n.is_power_of_two() {
        return Err("fft length must be a power of two");
    }
    // f64 复数工作区（re, im 分离向量，避免依赖）。
    let mut re: Vec<f64> = input.iter().map(|v| f64::from(*v)).collect();
    let mut im: Vec<f64> = vec![0.0; n];

    // 位反转重排。
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = (i.reverse_bits() >> (usize::BITS - bits)) & (n - 1);
        if j > i {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // 蝶形。
    let mut len = 2;
    while len <= n {
        let ang = -std::f64::consts::TAU / len as f64;
        let step_re = ang.cos();
        let step_im = ang.sin();
        let half = len / 2;
        for start in (0..n).step_by(len) {
            let mut w_re = 1.0f64;
            let mut w_im = 0.0f64;
            for k in 0..half {
                let i = start + k;
                let j = i + half;
                let tr = re[j] * w_re - im[j] * w_im;
                let ti = re[j] * w_im + im[j] * w_re;
                re[j] = re[i] - tr;
                im[j] = im[i] - ti;
                re[i] += tr;
                im[i] += ti;
                let w_new_re = w_re * step_re - w_im * step_im;
                w_im = w_re * step_im + w_im * step_re;
                w_re = w_new_re;
            }
        }
        len <<= 1;
    }
    Ok((
        re.into_iter().map(|v| v as f32).collect(),
        im.into_iter().map(|v| v as f32).collect(),
    ))
}

/// 幅度谱的前半段（含 DC，不含 Nyquist 以上）：`n/2+1` 点。
///
/// # Errors
/// 长度非 2 的幂。
pub fn magnitude_half(input: &[f32]) -> Result<Vec<f32>, &'static str> {
    let (re, im) = fft_forward(input)?;
    let half = input.len() / 2 + 1;
    Ok(re
        .into_iter()
        .zip(im)
        .take(half)
        .map(|(r, i)| (r * r + i * i).sqrt())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f32, b: f32, tol: f32, what: &str) {
        assert!((a - b).abs() < tol, "{what}: {a} vs {b}");
    }

    #[test]
    fn dc_only() {
        let (re, im) = fft_forward(&[1.0; 8]).unwrap();
        assert_close(re[0], 8.0, 1e-5, "DC = N");
        for v in &im {
            assert!(v.abs() < 1e-5);
        }
        for r in &re[1..] {
            assert!(r.abs() < 1e-5);
        }
    }

    #[test]
    fn single_tone_at_bin() {
        // 8 点 FFT，x[n] = exp(j·2π·2n/8) 的实部 = cos(2π·2n/8)。
        let n = 8;
        let x: Vec<f32> = (0..n)
            .map(|i| (std::f32::consts::TAU * 2.0 * i as f32 / n as f32).cos())
            .collect();
        let (re, im) = fft_forward(&x).unwrap();
        assert_close(re[2], 4.0, 1e-4, "bin2 实部 = N/2");
        assert_close(im[2], 0.0, 1e-4, "cos → 共轭对称虚部 0（bin2）");
        assert_close(re[6], 4.0, 1e-4, "共轭对称 bin6");
        for (k, r) in re.iter().enumerate() {
            if k != 2 && k != 6 {
                assert!(r.abs() < 1e-4, "bin{k} 应为 0");
            }
        }
    }

    #[test]
    fn magnitude_half_matches_dft_naive() {
        // 与朴素 DFT 对照（4 点任意输入）。
        let x = [1.0, 0.5, -0.25, 0.75];
        let mag = magnitude_half(&x).unwrap();
        let n = 4.0f32;
        for (k, m) in mag.iter().enumerate() {
            let mut re = 0.0f64;
            let mut im = 0.0f64;
            for (i, &v) in x.iter().enumerate() {
                let ang = -std::f64::consts::TAU * k as f64 * i as f64 / n as f64;
                re += f64::from(v) * ang.cos();
                im += f64::from(v) * ang.sin();
            }
            let expect = ((re * re + im * im).sqrt()) as f32;
            assert_close(*m, expect, 1e-4, "bin{k}");
        }
    }

    #[test]
    fn non_power_of_two_rejected() {
        assert!(fft_forward(&[1.0, 2.0, 3.0]).is_err());
        assert!(fft_forward(&[]).is_err());
    }
}
