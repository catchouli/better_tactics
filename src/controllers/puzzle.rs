use std::fmt::Display;
use askama::Template;
use axum::extract::Path;

use super::{BaseTemplateData, ControllerError};

/// The puzzle mode.
#[derive(Debug, PartialEq, Eq)]
pub enum PuzzleMode {
    /// We're showing a review.
    Review,

    /// We're showing a random new puzzle.
    Random,

    /// We're showing a specifically requested puzzle.
    Specific,
}

impl Display for PuzzleMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PuzzleMode::Review => write!(f, "Review"),
            PuzzleMode::Random => write!(f, "Random"),
            PuzzleMode::Specific => write!(f, "Specific"),
        }
    }
}

/// The template for displaying puzzles.
#[derive(Template)]
#[template(path = "puzzle.html")]
pub struct PuzzleTemplate {
    base: BaseTemplateData,
    mode: PuzzleMode,
    requested_id: String,
}

/// GET /tactics/by_id/{puzzle_id}
pub async fn specific_puzzle(Path(puzzle_id): Path<String>)
    -> Result<PuzzleTemplate, ControllerError>
{
    Ok(PuzzleTemplate {
        base: Default::default(),
        mode: PuzzleMode::Specific,
        requested_id: puzzle_id,
    })
}

/// GET /tactics/new
pub async fn random_puzzle()
    -> Result<PuzzleTemplate, ControllerError>
{
    Ok(PuzzleTemplate {
        base: Default::default(),
        mode: PuzzleMode::Random,
        requested_id: "".to_string(),
    })
}

/// GET /tactics
pub async fn next_review()
    -> Result<PuzzleTemplate, ControllerError>
{
    Ok(PuzzleTemplate {
        base: Default::default(),
        mode: PuzzleMode::Review,
        requested_id: "".to_string(),
    })
}
