use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct GapValue {
    pub pixels: f64,
    pub raw: String,
}

impl GapValue {
    pub fn from_ticks(t: u64) -> Self {
        Self { pixels: t as f64, raw: String::new() }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        if let Some(rest) = s.strip_suffix("rem") {
            let n: f64 = rest.parse().ok()?;
            Some(Self { pixels: n * 16.0, raw: String::from(s) })
        } else if let Some(rest) = s.strip_suffix("em") {
            let n: f64 = rest.parse().ok()?;
            Some(Self { pixels: n * 16.0, raw: String::from(s) })
        } else if let Some(rest) = s.strip_suffix("px") {
            let n: f64 = rest.parse().ok()?;
            Some(Self { pixels: n, raw: String::from(s) })
        } else {
            let n: f64 = s.parse().ok()?;
            Some(Self { pixels: n, raw: { let mut r = String::from(s); r.push_str("px"); r } })
        }
    }
}

#[derive(Debug, Clone)]
pub enum Constraint {
    // Temporal: run ordering
    After(String),
    Before(String),

    // Temporal: spacing
    GapAfter(GapValue, Option<String>),

    // Resource: sharing
    ShareMemory(String),
    ExcludeFrom(String),

    // Resource: matching
    MatchPriority(String),
    MatchAffinity(String),

    // Spatial (for layout — carried over from Outconceive)
    Left(String),
    Right(String),
    Top(String),
    Bottom(String),
    CenterX(String),
    CenterY(String),
    GapX(GapValue, Option<String>),
    GapY(GapValue, Option<String>),
    MatchWidth(String),
    MatchHeight(String),
}

impl Constraint {
    pub fn references(&self) -> Vec<&str> {
        match self {
            Self::After(r) | Self::Before(r)
            | Self::ShareMemory(r) | Self::ExcludeFrom(r)
            | Self::MatchPriority(r) | Self::MatchAffinity(r)
            | Self::Left(r) | Self::Right(r) | Self::Top(r) | Self::Bottom(r)
            | Self::CenterX(r) | Self::CenterY(r)
            | Self::MatchWidth(r) | Self::MatchHeight(r) => vec![r.as_str()],
            Self::GapAfter(_, Some(r))
            | Self::GapX(_, Some(r)) | Self::GapY(_, Some(r)) => vec![r.as_str()],
            Self::GapAfter(_, None)
            | Self::GapX(_, None) | Self::GapY(_, None) => vec![],
        }
    }

    pub fn is_temporal(&self) -> bool {
        matches!(self, Self::After(_) | Self::Before(_) | Self::GapAfter(..))
    }

    pub fn is_spatial(&self) -> bool {
        matches!(self,
            Self::Left(_) | Self::Right(_) | Self::Top(_) | Self::Bottom(_)
            | Self::CenterX(_) | Self::CenterY(_)
            | Self::GapX(..) | Self::GapY(..)
            | Self::MatchWidth(_) | Self::MatchHeight(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_after_references() {
        let c = Constraint::After(String::from("init"));
        assert_eq!(c.references(), vec!["init"]);
        assert!(c.is_temporal());
        assert!(!c.is_spatial());
    }

    #[test]
    fn test_spatial_constraint() {
        let c = Constraint::CenterX(String::from("header"));
        assert!(!c.is_temporal());
        assert!(c.is_spatial());
    }

    #[test]
    fn test_gap_no_ref() {
        let c = Constraint::GapAfter(GapValue::from_ticks(10), None);
        assert!(c.references().is_empty());
    }
}
