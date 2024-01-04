#![allow(unused)]

use anyhow::Result;
use thiserror::Error;

use std::collections::HashMap;

#[derive(Error, Debug)]
enum Err {
    #[error("Error: {0}")]
    Reason(String),
    #[error("Syntax error; line:{0} col:{1}")]
    SyntaxErr(u32, u32),
    #[error("Parens not balanced; {0} parens needed")]
    UnbalancedParens(usize),
}

#[derive(Clone, Debug, PartialEq)]
enum Exp {
    Symbol(String),
    Number(f64),
    List(Vec<Exp>),
    Func(fn(&[Exp]) -> Result<Exp>),
}

#[derive(Clone, Debug)]
struct Env {
    data: HashMap<String, Exp>,
}

fn tokenize(expr: String) -> Vec<String> {
    expr.replace("(", " ( ")
        .replace(")", " ) ")
        .split_whitespace()
        .map(|x| x.to_owned())
        .collect()
}

fn parse<'a>(tokens: &'a [String]) -> Result<(Exp, &'a [String])> {
    let (token, rest) = tokens
        .split_first()
        .ok_or(Err::Reason("Could not get token".to_owned()))?;
    match token.as_str() {
        "(" => read_seq(rest),
        ")" => Err(Err::Reason("Unexpected `)`".to_owned()).into()),
        _ => Ok((parse_atom(token), rest)),
    }
}

fn read_seq<'a>(tokens: &'a [String]) -> Result<(Exp, &'a [String])> {
    let mut result: Vec<Exp> = vec![];
    let mut xs = tokens;
    loop {
        let (next_token, rest) = xs
            .split_first()
            .ok_or(Err::Reason("Could not find closing `)`".to_owned()))?;
        if next_token == ")" {
            return Ok((Exp::List(result), rest));
        }
        let (exp, new_xs) = parse(&xs)?;
        result.push(exp);
        xs = new_xs;
    }
}

fn parse_atom(token: &str) -> Exp {
    let parse_result = token.parse();
    match parse_result {
        Ok(v) => Exp::Number(v),
        Err(_) => Exp::Symbol(token.to_owned()),
    }
}

fn default_env() -> Env {
    // `data` is a map from symbols to expressions
    let mut data = HashMap::<String, Exp>::new();
    data.insert(
        "+".to_owned(),
        Exp::Func(|args: &[Exp]| -> Result<Exp> {
            let floats = parse_list_of_floats(args)?;
            let sum: f64 = floats.iter().sum();
            Ok(Exp::Number(sum))
        }),
    );
    data.insert(
        "-".to_owned(),
        Exp::Func(|args: &[Exp]| -> Result<Exp> {
            let floats = parse_list_of_floats(args)?;
            let &first = floats
                .first()
                .ok_or(Err::Reason("`-` requires at least one operand".to_owned()))?;
            let sum_rest: f64 = floats.iter().skip(1).sum();
            Ok(Exp::Number(first - sum_rest))
        }),
    );
    Env { data }
}

fn parse_list_of_floats(floats: &[Exp]) -> Result<Vec<f64>> {
    floats.iter().map(|exp| parse_single_float(exp)).collect()
}

fn parse_single_float(exp: &Exp) -> Result<f64> {
    match exp {
        Exp::Number(num) => Ok(*num),
        _ => Err(Err::Reason("Expected a number".to_owned()))?,
    }
}

fn eval(exp: &Exp, env: &mut Env) -> Result<Exp> {
    match exp {
        // lookup symbol
        Exp::Symbol(symbol) => Ok(env
            .data
            .get(symbol)
            .ok_or(Err::Reason(format!("Unexpected symbol `{symbol}`")))
            .cloned()?),

        // return the number
        Exp::Number(_) => Ok(exp.clone()),

        // evaluate each item in list and apply
        Exp::List(list) => {
            // get car and cdr
            let (op, args) = list
                .split_first()
                .ok_or(Err::Reason("Expected non-empty list".to_owned()))?;

            // evaluate the operator
            let op = eval(op, env)?;

            // check that op is a function
            match op {
                Exp::Func(op) => {
                    // evaluate args
                    let args = args
                        .iter()
                        .map(|x| eval(x, env))
                        .collect::<Result<Vec<Exp>>>()?;

                    // apply
                    op(&args)
                }
                _ => Err(Err::Reason("Operator must be a function".to_owned()).into()),
            }
        }

        // shouldn't be allowed
        Exp::Func(_) => Err(Err::Reason("Cannot evaluate a function".to_owned()).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_error() {
        assert_eq!(
            "Error: Hello there".to_owned(),
            format!("{}", Err::Reason("Hello there".to_owned()))
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

    #[test]
    fn check_parse() {
        let lexemes = "(+ 1 2)".to_owned();
        let tokens = tokenize(lexemes);
        let (exp, rest) = parse(tokens.as_slice()).unwrap();
        assert_eq!(
            exp,
            Exp::List(vec![
                Exp::Symbol("+".to_owned()),
                Exp::Number(1.0),
                Exp::Number(2.0),
            ])
        );
        assert!(rest.is_empty());
    }

    #[test]
    fn check_parse_atom() {
        assert_eq!(parse_atom("1.0"), Exp::Number(1.0));
        assert_eq!(parse_atom("Hello"), Exp::Symbol("Hello".to_owned()));
        assert_eq!(parse_atom("hi1.0hi"), Exp::Symbol("hi1.0hi".to_owned()));
    }

    #[test]
    fn check_default_env() {
        let Env { data } = default_env();

        let add = *match data.get("+").unwrap() {
            Exp::Func(f) => f,
            _ => panic!("data did not return addition"),
        };
        let sub = *match data.get("-").unwrap() {
            Exp::Func(f) => f,
            _ => panic!("data did not return subtraction"),
        };

        let exps = vec![Exp::Number(1.0), Exp::Number(2.0), Exp::Number(3.0)];

        assert_eq!(add(&exps).unwrap(), Exp::Number(6.0));
        assert_eq!(sub(&exps).unwrap(), Exp::Number(-4.0));
    }

    #[test]
    fn check_eval() {
        let mut env = default_env();

        // Exp::List
        let (exp1, _) = parse(&tokenize("(+ 1 2)".to_owned())).unwrap();
        let (exp2, _) = parse(&tokenize("(+ 1 (+ 2 3 4))".to_owned())).unwrap();
        let (exp3, _) = parse(&tokenize("(- 2 3)".to_owned())).unwrap();
        let (exp4, _) = parse(&tokenize("(- 2 (+ 1 2 3))".to_owned())).unwrap();

        assert_eq!(eval(&exp1, &mut env).unwrap(), Exp::Number(3.0));
        assert_eq!(eval(&exp2, &mut env).unwrap(), Exp::Number(10.0));
        assert_eq!(eval(&exp3, &mut env).unwrap(), Exp::Number(-1.0));
        assert_eq!(eval(&exp4, &mut env).unwrap(), Exp::Number(-4.0));
    }
}
