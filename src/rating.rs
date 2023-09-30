// Implementation of Glicko3 ratings. I didn't fully understand everything, but I implemented the
// equations in the following pdf file, and it seems to resemble something of a rating system.
// https://en.wikipedia.org/wiki/Glicko_rating_system
// http://www.glicko.net/glicko/glicko2.pdf
use std::f64::consts::PI;

/// The constant tau constrains the volatility over time. Smaller values of tau prevent dramatic
/// rating changes after upset results.
const TAU: f64 = 0.2;

/// A struct representing a player's Glicko2 rating and rating deviation.
#[derive(Debug, Copy, Clone)]
pub struct Rating {
    pub rating: i64,
    pub deviation: i64,
    pub volatility: f64,
}

/// A single game result
#[derive(Debug, Copy, Clone)]
pub struct GameResult<T> {
    pub rating: T,
    pub deviation: T,
    /// The score (0 for a loss, 0.5 for a draw, or 1 for a win).
    pub score: f64,
}

impl Rating {
    /// Update the rating based on a list of game results over a rating period.
    pub fn update(&mut self, results: Vec<GameResult<i64>>) {
        // The conversion factor and offset for ratings to the glicko-2 scale.
        const RATING_SCALE: f64 = 173.7178;
        const RATING_OFFSET: f64 = 1500.0;

        // Convert the ratings onto the glicko-2 scale.
        let rating = (self.rating as f64 - RATING_OFFSET) / RATING_SCALE;
        let deviation = self.deviation as f64 / RATING_SCALE;

        let results: Vec<GameResult<f64>> = results.into_iter().map(|result| {
            GameResult {
                rating: (result.rating as f64 - RATING_OFFSET) / RATING_SCALE,
                deviation: result.deviation as f64 / RATING_SCALE,
                score: result.score,
            }
        }).collect();

        // Compute the quantity v, the estimated variance of the player's rating based only on game
        // outcomes.
        let variance = 1.0 / results.iter().map(|result| {
            let g = Self::g(result.deviation);
            let e = Self::e(rating, result.rating, result.deviation);

            g * g * e * (1.0 - e)
        }).sum::<f64>();

        // Compute the quantity delta, the estimated improvement in rating by comparing pre-period
        // rating to the performance rating based only on game outcomes.
        let delta = variance * results.iter().map(|result| {
            let g = Self::g(result.deviation);
            let e = Self::e(rating, result.rating, result.deviation);

            g * (result.score - e)
        }).sum::<f64>();

        // Determine the new value of the volatility.
        self.volatility = Self::calculate_new_volatility(deviation, variance, delta, self.volatility);

        // Update the rating deviation to the new pre-rating period value
        let deviation_pre = f64::sqrt(deviation * deviation + self.volatility * self.volatility);

        // Update the rating and RD to the new values
        let deviation_new = 1.0 / f64::sqrt(1.0 / (deviation_pre * deviation_pre) + 1.0 / variance);
        let rating_new = rating + (deviation_new * deviation_new * results.iter().map(|result| {
            let g = Self::g(result.deviation);
            let e = Self::e(rating, result.rating, result.deviation);

            g * (result.score - e)
        }).sum::<f64>());

        // Convert ratings and RDs back to original scale.
        self.rating = (rating_new * RATING_SCALE + RATING_OFFSET) as i64;
        self.deviation = (RATING_SCALE * deviation_new) as i64;
    }

    /// The iterative method to calculate the new volatility value from the paper. For the function
    /// f(x) defined in the paper, determine the value of x where f(x) = 0. (Don't ask me how this
    /// works though, because I don't really know. Hopefully it's correct enough though...)
    fn calculate_new_volatility(deviation: f64, variance: f64, delta: f64, volatility: f64) -> f64 {
        const EPSILON: f64 = 0.000001;

        // Define the function f(x) from the paper.
        let f = |x| {
            let a = f64::ln(volatility * volatility);
            let b = f64::exp(x) * (delta * delta - deviation * deviation - variance - f64::exp(x));
            let c = deviation * deviation + variance + f64::exp(x);

            let left = b / (2.0 * c * c);
            let right = (x - a) / (TAU * TAU);

            left - right
        };

        // Set the initial values of the iterative algorithm.
        let mut a = f64::ln(volatility * volatility);

        let mut b = if (delta * delta) > (deviation * deviation + variance) {
            f64::ln(delta * delta - deviation * deviation - variance)
        }
        else {
            let mut k: f64 = 1.0;

            while f(a - k * TAU) < 0.0 {
                k += 1.0;
            }

            a - k * TAU
        };

        let mut f_a = f(a);
        let mut f_b = f(b);

        while f64::abs(b - a) > EPSILON {
            let c = a + (a - b) * f_a / (f_b - f_a);
            let f_c = f(c);

            if (f_c * f_b) <= 0.0 {
                a = b;
                f_a = f_b;
            }
            else {
                f_a = f_a / 2.0;
            }

            b = c;
            f_b = f_c;
        }

        f64::exp(a / 2.0)
    }

    /// The function g from the paper.
    fn g(deviation: f64) -> f64 {
        1.0 / f64::sqrt(1.0 + (3.0 * deviation * deviation) / (PI * PI))
    }

    /// The function E from the paper.
    fn e(rating: f64, rating_other: f64, deviation_other: f64) -> f64 {
        1.0 / (1.0 + f64::exp(-Self::g(deviation_other) * (rating - rating_other)))
    }
}
