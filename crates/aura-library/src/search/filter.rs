//! 过滤表达式（§38/§39）：`field:value` / `field:>v` / `field:<v` /
//! `field:min-max`，空格分隔的 term 全部 AND。
//!
//! M3 拥有解析与 AST；求值只对已有字段生效。分析类字段
//! （bpm/key/energy 等，M8 才有数据）解析成功但求值返回
//! `FilterOutcome::MissingData` —— 缺失字段不猜值（§38）。

use std::fmt;

/// 过滤字段。已知字段白名单；未知字段是可恢复错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterField {
    Title,
    Artist,
    Album,
    AlbumArtist,
    Genre,
    Year,
    Codec,
    Container,
    TrackNumber,
    DiscNumber,
    /// 未来 M8 分析字段：M3 无数据。
    Bpm,
    MusicalKey,
    Energy,
}

impl FilterField {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "title" => Self::Title,
            "artist" => Self::Artist,
            "album" => Self::Album,
            "albumartist" | "album_artist" => Self::AlbumArtist,
            "genre" => Self::Genre,
            "year" => Self::Year,
            "codec" => Self::Codec,
            "container" => Self::Container,
            "track" | "tracknumber" => Self::TrackNumber,
            "disc" | "discnumber" => Self::DiscNumber,
            "bpm" => Self::Bpm,
            "key" => Self::MusicalKey,
            "energy" => Self::Energy,
            _ => return None,
        })
    }

    /// 分析类字段（M8 域；M3 无数据，不求值）。
    #[must_use]
    pub fn is_analysis(self) -> bool {
        matches!(self, Self::Bpm | Self::MusicalKey | Self::Energy)
    }
}

/// 比较操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    Eq,
    Gt,
    Lt,
    Range,
}

/// 值：文本或数值。
#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    Text(String),
    Number(f64),
}

/// 单个过滤条件。
#[derive(Debug, Clone, PartialEq)]
pub struct FilterTerm {
    pub field: FilterField,
    pub op: FilterOp,
    pub value: FilterValue,
    /// Range 的上界（仅 `op = Range`）。
    pub max: Option<FilterValue>,
}

/// 过滤表达式：term 序列（AND 语义）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterExpression {
    pub terms: Vec<FilterTerm>,
}

impl fmt::Display for FilterExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, t) in self.terms.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            let field = match t.field {
                FilterField::Title => "title",
                FilterField::Artist => "artist",
                FilterField::Album => "album",
                FilterField::AlbumArtist => "albumartist",
                FilterField::Genre => "genre",
                FilterField::Year => "year",
                FilterField::Codec => "codec",
                FilterField::Container => "container",
                FilterField::TrackNumber => "track",
                FilterField::DiscNumber => "disc",
                FilterField::Bpm => "bpm",
                FilterField::MusicalKey => "key",
                FilterField::Energy => "energy",
            };
            write!(f, "{field}:")?;
            match (&t.value, t.max.as_ref(), t.op) {
                (v, Some(mx), FilterOp::Range) => {
                    write!(f, "{}-{}", display_value(v), display_value(mx))?;
                }
                (FilterValue::Number(n), _, FilterOp::Gt) => write!(f, ">{n}")?,
                (FilterValue::Number(n), _, FilterOp::Lt) => write!(f, "<{n}")?,
                (v, _, _) => write!(f, "{}", display_value(v))?,
            }
        }
        Ok(())
    }
}

fn display_value(v: &FilterValue) -> String {
    match v {
        FilterValue::Text(s) => s.clone(),
        FilterValue::Number(n) => format!("{n}"),
    }
}

/// 解析错误（确定性、可恢复、无 panic，§39）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterParseError {
    /// 出错的原始 term。
    pub term: String,
    pub reason: &'static str,
}

impl fmt::Display for FilterParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid filter term {:?}: {}", self.term, self.reason)
    }
}

impl std::error::Error for FilterParseError {}

/// 解析过滤表达式。空输入 → 空 expression（匹配全部）。
///
/// # Errors
/// 未知字段 / 空值 / 非法数值 / 非法区间时返回 [`FilterParseError`]。
pub fn parse_filter(input: &str) -> Result<FilterExpression, FilterParseError> {
    let mut terms = Vec::new();
    for raw_term in input.split_whitespace() {
        let Some((field_raw, rest)) = raw_term.split_once(':') else {
            return Err(FilterParseError {
                term: raw_term.to_string(),
                reason: "expected field:value",
            });
        };
        let Some(field) = FilterField::parse(field_raw) else {
            return Err(FilterParseError {
                term: raw_term.to_string(),
                reason: "unknown field",
            });
        };
        if rest.is_empty() {
            return Err(FilterParseError {
                term: raw_term.to_string(),
                reason: "empty value",
            });
        }

        let (op, value_raw) = if let Some(v) = rest.strip_prefix('>') {
            (FilterOp::Gt, v)
        } else if let Some(v) = rest.strip_prefix('<') {
            (FilterOp::Lt, v)
        } else {
            (FilterOp::Eq, rest)
        };

        let (op, value, max) = if op == FilterOp::Eq {
            let (value, max) = parse_range_or_eq(value_raw, raw_term)?;
            let actual = if max.is_some() {
                FilterOp::Range
            } else {
                FilterOp::Eq
            };
            (actual, value, max)
        } else {
            (op, parse_value(value_raw, raw_term)?, None)
        };

        // 数值比较操作只接受数值。
        if matches!(op, FilterOp::Gt | FilterOp::Lt) && !matches!(value, FilterValue::Number(_)) {
            return Err(FilterParseError {
                term: raw_term.to_string(),
                reason: ">/&lt; require numeric value",
            });
        }

        terms.push(FilterTerm {
            field,
            op,
            value,
            max,
        });
    }
    Ok(FilterExpression { terms })
}

/// `min-max` 区间或等值。
fn parse_range_or_eq(
    raw: &str,
    term: &str,
) -> Result<(FilterValue, Option<FilterValue>), FilterParseError> {
    if let Some((min_raw, max_raw)) = raw.split_once('-') {
        if min_raw.is_empty() || max_raw.is_empty() {
            return Err(FilterParseError {
                term: term.to_string(),
                reason: "empty range bound",
            });
        }
        let min = parse_value(min_raw, term)?;
        let max = parse_value(max_raw, term)?;
        return Ok((min, Some(max)));
    }
    Ok((parse_value(raw, term)?, None))
}

fn parse_value(raw: &str, term: &str) -> Result<FilterValue, FilterParseError> {
    if let Ok(n) = raw.parse::<f64>() {
        if n.is_finite() {
            return Ok(FilterValue::Number(n));
        }
    }
    // 文本值：禁止空白与控制字符（term 已按空白切分，这里防注入类字符）。
    if raw
        .chars()
        .all(|c| !c.is_control() && c != '"' && c != '\\' && c != '(' && c != ')')
    {
        Ok(FilterValue::Text(raw.to_string()))
    } else {
        Err(FilterParseError {
            term: term.to_string(),
            reason: "invalid characters in text value",
        })
    }
}

/// 求值输入：track 的字段视图。
#[derive(Debug, Clone, Default)]
pub struct FilterContext<'a> {
    pub title: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    pub album_artist: Option<&'a str>,
    pub genre: Option<&'a str>,
    pub year: Option<i32>,
    pub codec: Option<&'a str>,
    pub container: Option<&'a str>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
}

/// 求值结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOutcome {
    Match,
    NoMatch,
    /// 字段属于 M8 分析域，M3 无数据：不猜值（§38）。
    MissingData,
}

/// 对单个 track 求值（AND 语义；文本比较走 search_text 规范化）。
#[must_use]
pub fn evaluate(expr: &FilterExpression, ctx: &FilterContext<'_>) -> FilterOutcome {
    let mut any_missing = false;
    for term in &expr.terms {
        if term.field.is_analysis() {
            any_missing = true;
            continue;
        }
        let matched = match (term.field, &term.value) {
            (FilterField::Title, FilterValue::Text(v)) => ctx.title.is_some_and(|s| {
                crate::normalize::search_text(s).contains(&crate::normalize::search_text(v))
            }),
            (FilterField::Artist, FilterValue::Text(v)) => ctx.artist.is_some_and(|s| {
                crate::normalize::search_text(s).contains(&crate::normalize::search_text(v))
            }),
            (FilterField::Album, FilterValue::Text(v)) => ctx.album.is_some_and(|s| {
                crate::normalize::search_text(s).contains(&crate::normalize::search_text(v))
            }),
            (FilterField::AlbumArtist, FilterValue::Text(v)) => ctx.album_artist.is_some_and(|s| {
                crate::normalize::search_text(s).contains(&crate::normalize::search_text(v))
            }),
            (FilterField::Genre, FilterValue::Text(v)) => ctx.genre.is_some_and(|s| {
                crate::normalize::search_text(s).contains(&crate::normalize::search_text(v))
            }),
            (FilterField::Codec, FilterValue::Text(v)) => {
                ctx.codec.is_some_and(|s| s.eq_ignore_ascii_case(v))
            }
            (FilterField::Container, FilterValue::Text(v)) => {
                ctx.container.is_some_and(|s| s.eq_ignore_ascii_case(v))
            }
            (FilterField::Year, FilterValue::Number(n)) => {
                op_holds(ctx.year.map(f64::from), *n, term.op, term.max.as_ref())
            }
            (FilterField::TrackNumber, FilterValue::Number(n)) => op_holds(
                ctx.track_number.map(f64::from),
                *n,
                term.op,
                term.max.as_ref(),
            ),
            (FilterField::DiscNumber, FilterValue::Number(n)) => op_holds(
                ctx.disc_number.map(f64::from),
                *n,
                term.op,
                term.max.as_ref(),
            ),
            _ => false,
        };
        if !matched {
            // 文本字段缺失 = 不匹配（非 missing-data 语义：这些字段解析后为
            // None 的 track 对文本过滤天然不命中）。
            return FilterOutcome::NoMatch;
        }
    }
    if any_missing {
        FilterOutcome::MissingData
    } else {
        FilterOutcome::Match
    }
}

fn op_holds(actual: Option<f64>, target: f64, op: FilterOp, max: Option<&FilterValue>) -> bool {
    let Some(a) = actual else {
        return false;
    };
    match (op, max) {
        (FilterOp::Range, Some(FilterValue::Number(mx))) => a >= target && a <= *mx,
        (FilterOp::Gt, _) => a > target,
        (FilterOp::Lt, _) => a < target,
        _ => (a - target).abs() < f64::EPSILON * 8.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §39：基础语法。
    #[test]
    fn parse_basic_forms() {
        let f = parse_filter("year:1999").unwrap();
        assert_eq!(f.terms.len(), 1);
        assert_eq!(f.terms[0].op, FilterOp::Eq);

        let f = parse_filter("bpm:120-130").unwrap();
        assert!(f.terms[0].field.is_analysis());
        assert_eq!(f.terms[0].op, FilterOp::Range);
        assert_eq!(f.terms[0].value, FilterValue::Number(120.0));
        assert_eq!(f.terms[0].max, Some(FilterValue::Number(130.0)));

        let f = parse_filter("energy:>0.6").unwrap();
        assert_eq!(f.terms[0].op, FilterOp::Gt);

        let f = parse_filter("year:<1980").unwrap();
        assert_eq!(f.terms[0].op, FilterOp::Lt);

        let f = parse_filter("key:8A").unwrap();
        assert_eq!(f.terms[0].value, FilterValue::Text("8A".into()));
    }

    #[test]
    fn parse_multi_term_and() {
        let f = parse_filter("genre:rock year:>1990").unwrap();
        assert_eq!(f.terms.len(), 2);
    }

    /// §39：可恢复错误、清晰原因、无 panic。
    #[test]
    fn parse_errors_are_clear() {
        for bad in [
            "nofield",
            "bpm:",
            "bpm:abc-",
            "year:>abc",
            "a:b:c",
            "year:1-",
        ] {
            let err = parse_filter(bad).unwrap_err();
            assert!(!err.reason.is_empty());
            assert_eq!(
                err.term.split_whitespace().next().unwrap_or(""),
                err.term.split(' ').next().unwrap_or("")
            );
        }
        // 空输入合法（匹配全部）。
        assert!(parse_filter("").unwrap().terms.is_empty());
    }

    /// round-trip：Display 输出可再次解析为等价 AST（确定性）。
    #[test]
    fn display_round_trip() {
        let src = "year:1999 bpm:120-130 energy:>0.6 genre:rock key:8A";
        let f1 = parse_filter(src).unwrap();
        let f2 = parse_filter(&f1.to_string()).unwrap();
        assert_eq!(f1, f2);
    }

    fn ctx<'a>(title: &'a str, year: i32) -> FilterContext<'a> {
        FilterContext {
            title: Some(title),
            year: Some(year),
            ..Default::default()
        }
    }

    #[test]
    fn evaluate_owned_fields() {
        let f = parse_filter("title:night year:1999-2005").unwrap();
        assert_eq!(evaluate(&f, &ctx("Nightcall", 2001)), FilterOutcome::Match);
        assert_eq!(evaluate(&f, &ctx("Daycall", 2001)), FilterOutcome::NoMatch);
        assert_eq!(
            evaluate(&f, &ctx("Nightcall", 1990)),
            FilterOutcome::NoMatch
        );
        // year 缺失 → 数值条件不命中（不是 missing-data：year 是 M3 字段）。
        assert_eq!(
            evaluate(
                &f,
                &FilterContext {
                    title: Some("Nightcall"),
                    ..Default::default()
                }
            ),
            FilterOutcome::NoMatch
        );
    }

    /// §38：分析字段缺失 → MissingData，不猜值。
    #[test]
    fn evaluate_analysis_fields_missing_data() {
        let f = parse_filter("energy:>0.6").unwrap();
        assert_eq!(evaluate(&f, &ctx("Any", 2000)), FilterOutcome::MissingData);
        let f2 = parse_filter("title:night energy:>0.6").unwrap();
        assert_eq!(
            evaluate(&f2, &ctx("Nightcall", 2000)),
            FilterOutcome::MissingData
        );
        assert_eq!(evaluate(&f2, &ctx("Daycall", 2000)), FilterOutcome::NoMatch);
    }
}
