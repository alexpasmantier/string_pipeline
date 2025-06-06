use pest::Parser;
use pest_derive::Parser;

use super::{RangeSpec, StringOp};

#[derive(Parser)]
#[grammar = "pipeline/template.pest"]
struct TemplateParser;

pub fn parse_template(template: &str) -> Result<(Vec<StringOp>, bool), String> {
    let pairs = TemplateParser::parse(Rule::template, template)
        .map_err(|e| format!("Parse error: {}", e))?
        .next()
        .unwrap();

    let mut ops = Vec::new();
    let mut debug = false;

    for pair in pairs.into_inner() {
        match pair.as_rule() {
            Rule::operation_list => {
                for op_pair in pair.into_inner() {
                    ops.push(parse_operation(op_pair)?);
                }
            }
            Rule::debug_flag => {
                debug = true;
            }
            _ => {}
        }
    }

    Ok((ops, debug))
}

fn parse_operation(pair: pest::iterators::Pair<Rule>) -> Result<StringOp, String> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::shorthand_range => {
            let range = parse_range_spec(inner)?;
            Ok(StringOp::Split {
                sep: " ".to_string(),
                range,
            })
        }
        Rule::shorthand_index => {
            let idx = inner.as_str().parse().unwrap();
            Ok(StringOp::Split {
                sep: " ".to_string(),
                range: RangeSpec::Index(idx),
            })
        }
        Rule::split => {
            let mut parts = inner.into_inner();
            let sep = process_arg(parts.next().unwrap().as_str());
            let range = parts
                .next()
                .map_or_else(|| Ok(RangeSpec::Range(None, None, false)), parse_range_spec)?;
            Ok(StringOp::Split { sep, range })
        }
        Rule::join => {
            let sep = process_arg(inner.into_inner().next().unwrap().as_str());
            Ok(StringOp::Join { sep })
        }
        Rule::substring => {
            let range = parse_range_spec(inner.into_inner().next().unwrap())?;
            Ok(StringOp::Substring { range })
        }
        Rule::replace => {
            let sed_parts = parse_sed_string(inner.into_inner().next().unwrap())?;
            Ok(StringOp::Replace {
                pattern: sed_parts.0,
                replacement: sed_parts.1,
                flags: sed_parts.2,
            })
        }
        Rule::upper => Ok(StringOp::Upper),
        Rule::lower => Ok(StringOp::Lower),
        Rule::trim => Ok(StringOp::Trim),
        Rule::strip => {
            let chars = inner.into_inner().next().unwrap().as_str().to_string();
            Ok(StringOp::Strip { chars })
        }
        Rule::append => {
            let suffix = process_arg(inner.into_inner().next().unwrap().as_str());
            Ok(StringOp::Append { suffix })
        }
        Rule::prepend => {
            let prefix = process_arg(inner.into_inner().next().unwrap().as_str());
            Ok(StringOp::Prepend { prefix })
        }
        Rule::strip_ansi => Ok(StringOp::StripAnsi),
        Rule::filter => {
            let pattern = inner.into_inner().next().unwrap().as_str().to_string();
            Ok(StringOp::Filter { pattern })
        }
        Rule::filter_not => {
            let pattern = inner.into_inner().next().unwrap().as_str().to_string();
            Ok(StringOp::FilterNot { pattern })
        }
        Rule::slice => {
            let range = parse_range_spec(inner.into_inner().next().unwrap())?;
            Ok(StringOp::Slice { range })
        }
        _ => Err(format!("Unknown operation: {:?}", inner.as_rule())),
    }
}

fn process_arg(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('n') => {
                    result.push('\n');
                    chars.next();
                }
                Some('t') => {
                    result.push('\t');
                    chars.next();
                }
                Some('r') => {
                    result.push('\r');
                    chars.next();
                }
                Some(':') => {
                    result.push(':');
                    chars.next();
                }
                Some('|') => {
                    result.push('|');
                    chars.next();
                }
                Some('\\') => {
                    result.push('\\');
                    chars.next();
                }
                Some('/') => {
                    result.push('/');
                    chars.next();
                }
                Some('{') => {
                    result.push('{');
                    chars.next();
                }
                Some('}') => {
                    result.push('}');
                    chars.next();
                }
                Some(&next) => {
                    result.push(next);
                    chars.next();
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn parse_sed_string(pair: pest::iterators::Pair<Rule>) -> Result<(String, String, String), String> {
    let mut parts = pair.into_inner();

    let pattern = parts.next().unwrap().as_str().to_string();
    let replacement = parts.next().unwrap().as_str().to_string();
    let flags = parts
        .next()
        .map_or_else(String::new, |p| p.as_str().to_string());

    if pattern.is_empty() {
        return Err("Empty pattern in sed string".to_string());
    }

    Ok((pattern, replacement, flags))
}

fn parse_range_spec(pair: pest::iterators::Pair<Rule>) -> Result<RangeSpec, String> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::range_inclusive => {
            let mut parts = inner.into_inner();
            let start = parts.next().and_then(|p| p.as_str().parse().ok());
            let end = parts.next().and_then(|p| p.as_str().parse().ok());
            Ok(RangeSpec::Range(start, end, true))
        }
        Rule::range_exclusive => {
            let mut parts = inner.into_inner();
            let start = parts.next().and_then(|p| p.as_str().parse().ok());
            let end = parts.next().and_then(|p| p.as_str().parse().ok());
            Ok(RangeSpec::Range(start, end, false))
        }
        Rule::range_from => {
            let start = inner.into_inner().next().unwrap().as_str().parse().ok();
            Ok(RangeSpec::Range(start, None, false))
        }
        Rule::range_to => {
            let end = inner.into_inner().next().unwrap().as_str().parse().ok();
            Ok(RangeSpec::Range(None, end, false))
        }
        Rule::range_to_inclusive => {
            let end = inner.into_inner().next().unwrap().as_str().parse().ok();
            Ok(RangeSpec::Range(None, end, true))
        }
        Rule::range_full => Ok(RangeSpec::Range(None, None, false)),
        Rule::index => {
            let idx = inner.into_inner().next().unwrap().as_str().parse().unwrap();
            Ok(RangeSpec::Index(idx))
        }
        _ => Err(format!("Unknown range spec: {:?}", inner.as_rule())),
    }
}
