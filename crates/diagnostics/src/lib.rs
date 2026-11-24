use ariadne::{Color, Label, Report, ReportKind, Source};
use std::ops::Range;

/// Represents a source code location span [start, end) byte indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn to_range(&self) -> Range<usize> {
        self.start..self.end
    }

    pub fn merge(&self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl From<Range<usize>> for Span {
    fn from(r: Range<usize>) -> Self {
        Span {
            start: r.start,
            end: r.end,
        }
    }
}

/// A structured diagnostic message conforming to Syntra specifications.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    pub label: Option<String>,
    pub note: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
            label: None,
            note: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Render this diagnostic to standard error using Ariadne formatting.
    pub fn emit(&self, filename: &str, source: &str) {
        let mut report = Report::build(ReportKind::Error, filename, self.span.start)
            .with_code(self.code)
            .with_message(&self.message);

        let label_text = self.label.as_deref().unwrap_or(&self.message);
        report.add_label(
            Label::new((filename, self.span.to_range()))
                .with_message(label_text)
                .with_color(Color::Red),
        );

        if let Some(note) = &self.note {
            report.set_note(note);
        }

        let _ = report.finish().eprint((filename, Source::from(source)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation_and_merge() {
        let s1 = Span::new(5, 10);
        let s2 = Span::new(8, 20);
        let merged = s1.merge(s2);
        assert_eq!(merged.start, 5);
        assert_eq!(merged.end, 20);
        assert_eq!(merged.to_range(), 5..20);

        let from_range = Span::from(3..15);
        assert_eq!(from_range.start, 3);
        assert_eq!(from_range.end, 15);
    }

    #[test]
    fn test_diagnostic_builder_and_fields() {
        let span = Span::new(10, 25);
        let diag = Diagnostic::error("DIAG-001", "Invalid token encountered", span)
            .with_label("unexpected identifier here")
            .with_note("consider using 'func' instead of 'def'");

        assert_eq!(diag.code, "DIAG-001");
        assert_eq!(diag.message, "Invalid token encountered");
        assert_eq!(diag.span, span);
        assert_eq!(diag.label, Some("unexpected identifier here".to_string()));
        assert_eq!(
            diag.note,
            Some("consider using 'func' instead of 'def'".to_string())
        );
    }
}
