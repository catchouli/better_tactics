// Implementation of Glicko2 ratings.
// https://en.wikipedia.org/wiki/Glicko_rating_system
// http://www.glicko.net/glicko/glicko2.pdf

use std::f64::consts::PI;

/// The constant tau constrains the volatility over time. Smaller values of tau prevent dramatic
/// rating changes after upset results.
const TAU: f64 = 0.05;

/// A struct representing a player's Glicko2 rating and rating deviation.
#[derive(Debug, Copy, Clone)]
pub struct Rating {
    pub rating: i64,
    pub deviation: i64,
    pub volatility: f64,
}

/// A single game result
#[derive(Debug, Copy, Clone)]
pub struct GameResult {
    pub rating: i64,
    pub deviation: i64,
    /// The score (0 for a loss, 0.5 for a draw, or 1 for a win).
    pub score: f64,
}

impl Rating {
    /// Update the rating based on a list of game results over a rating period.
    pub fn update(&mut self, results: Vec<GameResult>) {
        // Convert the ratings onto the glicko-2 scale.
        let mu = (self.rating - 1500) as f64 / 173.7178;
        let phi = self.deviation as f64 / 173.7178;

        log::info!("Old rating: {mu}, old deviation: {phi}");

        // Compute the quantity v, the estimated variance of the player's rating based only on game
        // outcomes.
        let variance = 1.0 / results.iter().map(|result| {
            let mu_other = (result.rating - 1500) as f64 / 173.7178;
            let phi_other = result.deviation as f64 / 173.7178;

            let g = Self::g(phi_other);
            let e = Self::e(mu, mu_other, phi_other);

            g * g * e * (1.0 - e)
        }).sum::<f64>();

        log::info!("Estimated variance {variance}");

        // Compute the quantity delta, the estimated improvement in rating by comparing pre-period
        // rating to the performance rating based only on game outcomes.
        let delta = variance * results.iter().map(|result| {
            let mu_other = (result.rating - 1500) as f64 / 173.7178;
            let phi_other = result.deviation as f64 / 173.7178;

            let g = Self::g(phi_other);
            let e = Self::e(mu, mu_other, phi_other);

            g * (result.score - e)
        }).sum::<f64>();

        log::info!("Delta: {delta}");

        // Determine the new value of the volatility.
        // wtf..
        const epsilon: f64 = 0.000001;

        let f = |x| {
            let a = f64::ln(self.volatility * self.volatility);
            let b = f64::exp(x) * (delta * delta - phi * phi - variance - f64::exp(x));
            let c = phi * phi + variance + f64::exp(x);

            let left = b / (2.0 * c * c);
            let right = (x - a) / (TAU * TAU);

            left - right
        };

        let mut A = f64::ln(self.volatility * self.volatility);

        let mut B = if (delta * delta) > (phi * phi + variance) {
            f64::ln(delta * delta - phi * phi - variance)
        }
        else {
            let mut k: f64 = 1.0;

            while f(A - k * TAU) < 0.0 {
                k += 1.0;
            }

            A - k * TAU
        };

        let mut fA = f(A);
        let mut fB = f(B);

        while f64::abs(B - A) > epsilon {
            let C = A + (A - B) * fA / (fB - fA);
            let fC = f(C);

            if (fC * fB) <= 0.0 {
                A = B;
                fA = fB;
            }
            else {
                fA = fA / 2.0;
            }

            B = C;
            fB = fC;
        }

        self.volatility = f64::exp(A / 2.0);

        log::info!("New volatility: {}", self.volatility);

        // Update the rating deviation to the new pre-rating period value
        let phi_pre = f64::sqrt(phi * phi + self.volatility * self.volatility);

        log::info!("New pre-rating period deviation: {}", phi_pre);

        // Update the rating and RD to the new values
        let phi_new = 1.0 / f64::sqrt(1.0 / (phi_pre * phi_pre) + 1.0 / variance);
        let mu_new = mu + (phi_new * phi_new * results.iter().map(|result| {
            let mu_other = (result.rating - 1500) as f64 / 173.7178;
            let phi_other = result.deviation as f64 / 173.7178;

            log::info!("Puzzle rating: {}, deviation: {}", mu_other, phi_other);

            let g = Self::g(phi_other);
            let e = Self::e(mu, mu_other, phi_other);

            g * (result.score - e)
        }).sum::<f64>());

        log::info!("New rating: {}, deviation: {}", mu_new, phi_new);

        // Convert ratings and RDs back to original scale.
        self.rating = (mu_new * 173.7178 + 1500.0) as i64;
        self.deviation = (173.7178 * phi_new) as i64;

        // Convert the 'other players' ratings too.
        //let results = results.into_iter().map(|result| GameResult {
        //    rating: (result.rating - 1500) as f64 / 173.7178,
        //    deviation: result.deviation as f64 / 173.7178,
        //    score: result.score,
        //}).collect();


    }

    /// The function g from the paper.
    fn g(phi: f64) -> f64 {
        1.0 / f64::sqrt(1.0 + (3.0 * phi * phi) / (PI * PI))
    }

    /// The function E from the paper.
    fn e(mu: f64, mu_other: f64, phi_other: f64) -> f64 {
        1.0 / (1.0 + f64::exp(-Self::g(phi_other) * (mu - mu_other)))
    }
}
