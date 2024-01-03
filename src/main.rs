use anyhow::Result;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
enum LargoErr {
    #[error("Error: {0}")]
    Reason(String),
    /// Syntax error returning line number and char
    #[error("Syntax error; line:{0} col:{1}")]
    SyntaxErr(u32, u32),
    /// Parens not balanced; contains number of parens needed
    #[error("Parens not balanced; {0} parens needed")]
    UnbalancedParens(usize),
}

#[derive(Clone)]
enum LargoExp {
    Symbol(String),
    Number(f64),
    List(Vec<LargoExp>),
}

#[derive(Clone)]
struct LargoEnv {
    data: HashMap<String, LargoExp>,
}

fn main() {}

fn tokenize(expr: String) -> Vec<String> {
    expr.replace("(", " ( ")
        .replace(")", " ) ")
        .split_whitespace()
        .map(|x| x.to_owned())
        .collect()
}

fn parse<'a>(tokens: &'a [String]) -> Result<(LargoExp, &'a [String])> {
    let (token, rest) = tokens
        .split_first()
        .ok_or(LargoErr::Reason("could not get token".to_owned()))?;
    match token.as_str() {
        "(" => read_seq(rest),
        ")" => Err(LargoErr::Reason("Unexpected `)`".to_owned()).into()),
        _ => Ok((parse_atom(token), rest)),
    }
}

fn read_seq<'a>(tokens: &'a [String]) -> Result<(LargoExp, &'a [String])> {
    let mut result: Vec<LargoExp> = vec![];
    let mut xs = tokens;
    loop {
        let (next_token, rest) = xs
            .split_first()
            .ok_or(LargoErr::Reason("Could not find closing `)`".to_owned()))?;
        if next_token == ")" {
            return Ok((LargoExp::List(result), rest));
        }
        let (exp, new_xs) = parse(&rest)?;
        result.push(exp);
        xs = new_xs;
    }
}

fn parse_atom(atom: &str) -> LargoExp {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_error_value() {
        assert_eq!(
            "Error: Hello there".to_owned(),
            format!("{}", LargoErr::Reason("Hello there".to_owned()))
        );
    }

    #[test]
    fn check_tokenize() {
        assert_eq!(
            tokenize("(+ 1 2)".to_owned()),
            vec![
                "(".to_owned(),
                "+".to_owned(),
                "1".to_owned(),
                "2".to_owned(),
                ")".to_owned()
            ]
        );
    }
}
