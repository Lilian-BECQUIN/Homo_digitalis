//! Intégrateur Runge-Kutta d'ordre 4 à pas fixe, générique sur la dimension du système.
//!
//! Le système d'EDO est `dy/dt = f(t, y)`. Les tableaux sont de taille statique
//! (`const N`) : aucune allocation dans la boucle chaude.

/// Un système d'équations différentielles ordinaires autonome ou non.
pub trait OdeSystem<const N: usize> {
    /// Évalue `f(t, y)`.
    fn derivatives(&self, t: f64, y: &[f64; N]) -> [f64; N];
}

#[inline]
fn axpy<const N: usize>(y: &[f64; N], a: f64, k: &[f64; N]) -> [f64; N] {
    let mut out = [0.0; N];
    for i in 0..N {
        out[i] = y[i] + a * k[i];
    }
    out
}

/// Un pas RK4 classique : erreur locale en O(dt⁵), globale en O(dt⁴).
pub fn rk4_step<const N: usize, S: OdeSystem<N>>(
    system: &S,
    t: f64,
    y: &[f64; N],
    dt: f64,
) -> [f64; N] {
    let k1 = system.derivatives(t, y);
    let k2 = system.derivatives(t + 0.5 * dt, &axpy(y, 0.5 * dt, &k1));
    let k3 = system.derivatives(t + 0.5 * dt, &axpy(y, 0.5 * dt, &k2));
    let k4 = system.derivatives(t + dt, &axpy(y, dt, &k3));

    let mut out = [0.0; N];
    for i in 0..N {
        out[i] = y[i] + dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// dy/dt = -λ y  →  y(t) = y0 · exp(-λ t)
    struct Decay(f64);
    impl OdeSystem<1> for Decay {
        fn derivatives(&self, _t: f64, y: &[f64; 1]) -> [f64; 1] {
            [-self.0 * y[0]]
        }
    }

    /// Oscillateur harmonique : y'' = -ω² y
    struct Oscillator;
    impl OdeSystem<2> for Oscillator {
        fn derivatives(&self, _t: f64, y: &[f64; 2]) -> [f64; 2] {
            [y[1], -y[0]]
        }
    }

    #[test]
    fn exponential_decay_matches_analytic_solution() {
        let sys = Decay(2.0);
        let mut y = [1.0];
        let dt = 0.01;
        let mut t = 0.0;
        for _ in 0..100 {
            y = rk4_step(&sys, t, &y, dt);
            t += dt;
        }
        let exact = (-2.0f64 * 1.0).exp();
        assert!((y[0] - exact).abs() < 1e-8, "y={} exact={}", y[0], exact);
    }

    #[test]
    fn fourth_order_convergence() {
        let sys = Decay(1.0);
        let err = |dt: f64| {
            let n = (1.0 / dt).round() as usize;
            let mut y = [1.0];
            for i in 0..n {
                y = rk4_step(&sys, i as f64 * dt, &y, dt);
            }
            (y[0] - (-1.0f64).exp()).abs()
        };
        let ratio = err(0.1) / err(0.05);
        // Ordre 4 → rapport théorique 16.
        assert!(ratio > 12.0 && ratio < 20.0, "ratio={ratio}");
    }

    #[test]
    fn oscillator_conserves_energy_approximately() {
        let mut y = [1.0, 0.0];
        let dt = 0.001;
        for i in 0..20_000 {
            y = rk4_step(&Oscillator, i as f64 * dt, &y, dt);
        }
        let energy = 0.5 * (y[0] * y[0] + y[1] * y[1]);
        assert!((energy - 0.5).abs() < 1e-6, "energy={energy}");
    }
}
