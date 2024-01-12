use anyhow::Result;
use thiserror::Error;

use std::collections::HashMap;
use std::io::Write;
use std::{fmt, io};

#[derive(Error, Debug)]
enum Error {
    #[error("{0}")]
    Reason(String),
    // #[error("Syntax error; line:{0} col:{1}")]
    // SyntaxErr(u32, u32),
    // #[error("Parens not balanced; {0} parens needed")]
    // UnbalancedParens(usize),
}

#[derive(Clone, Debug, PartialEq)]
enum Expr {
    Symbol(String),
    Number(f64),
    List(Vec<Expr>),
    Func(fn(&[Expr]) -> Result<Expr>),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let repr = match self {
            Expr::Symbol(s) => s.clone(),
            Expr::Number(n) => n.to_string(),
            Expr::List(l) => {
                let l: Vec<String> = l.iter().map(|exp| exp.to_string()).collect();
                format!("({})", l.join(","))
            }
            Expr::Func(_) => "Function".to_owned(),
        };
        write!(f, "{}", repr)
    }
}

#[derive(Clone, Debug)]
struct Env {
    data: HashMap<String, Expr>,
}

fn tokenize(expr: String) -> Vec<String> {
    expr.replace("(", " ( ")
        .replace(")", " ) ")
        .split_whitespace()
        .map(|x| x.to_owned())
        .collect()
}

fn parse<'a>(tokens: &'a [String]) -> Result<(Expr, &'a [String])> {
    let (token, rest) = tokens
        .split_first()
        .ok_or(Error::Reason("Could not get token".to_owned()))?;
    match token.as_str() {
        "(" => read_seq(rest),
        ")" => Err(Error::Reason("Unexpected `)`".to_owned()).into()),
        _ => Ok((parse_atom(token), rest)),
    }
}

fn read_seq<'a>(tokens: &'a [String]) -> Result<(Expr, &'a [String])> {
    let mut result: Vec<Expr> = vec![];
    let mut xs = tokens;
    loop {
        let (next_token, rest) = xs
            .split_first()
            .ok_or(Error::Reason("Could not find closing `)`".to_owned()))?;
        if next_token == ")" {
            return Ok((Expr::List(result), rest));
        }
        let (exp, new_xs) = parse(&xs)?;
        result.push(exp);
        xs = new_xs;
    }
}

fn parse_atom(token: &str) -> Expr {
    let parse_result = token.parse();
    match parse_result {
        Ok(v) => Expr::Number(v),
        Err(_) => Expr::Symbol(token.to_owned()),
    }
}

fn default_env() -> Env {
    // `data` is a map from symbols to expressions
    let mut data = HashMap::<String, Expr>::new();
    data.insert(
        "+".to_owned(),
        Expr::Func(|args: &[Expr]| -> Result<Expr> {
            let floats = parse_list_of_floats(args)?;
            let sum: f64 = floats.iter().sum();
            Ok(Expr::Number(sum))
        }),
    );
    data.insert(
        "-".to_owned(),
        Expr::Func(|args: &[Expr]| -> Result<Expr> {
            let floats = parse_list_of_floats(args)?;
            let &first = floats
                .first()
                .ok_or(Error::Reason("`-` requires at least one operand".to_owned()))?;
            let sum_rest: f64 = floats.iter().skip(1).sum();
            Ok(Expr::Number(first - sum_rest))
        }),
    );
    Env { data }
}

fn parse_list_of_floats(floats: &[Expr]) -> Result<Vec<f64>> {
    floats.iter().map(|exp| parse_single_float(exp)).collect()
}

fn parse_single_float(exp: &Expr) -> Result<f64> {
    match exp {
        Expr::Number(num) => Ok(*num),
        _ => Err(Error::Reason("Expected a number".to_owned()))?,
    }
}

fn eval(exp: &Expr, env: &mut Env) -> Result<Expr> {
    match exp {
        // lookup symbol
        Expr::Symbol(symbol) => Ok(env
            .data
            .get(symbol)
            .ok_or(Error::Reason(format!("Unexpected symbol `{symbol}`")))
            .cloned()?),

        // return the number
        Expr::Number(_) => Ok(exp.clone()),

        // evaluate each item in list and apply
        Expr::List(list) => {
            // get car and cdr
            let (op, args) = list
                .split_first()
                .ok_or(Error::Reason("Expected non-empty list".to_owned()))?;

            // evaluate the operator
            let op = eval(op, env)?;

            // check that op is a function
            match op {
                Expr::Func(op) => {
                    // evaluate args
                    let args = args
                        .iter()
                        .map(|x| eval(x, env))
                        .collect::<Result<Vec<Expr>>>()?;

                    // apply
                    op(&args)
                }
                _ => Err(Error::Reason("Operator must be a function".to_owned()).into()),
            }
        }

        // shouldn't be allowed
        Expr::Func(_) => Err(Error::Reason("Cannot evaluate a function".to_owned()).into()),
    }
}

fn string_to_exp(lexemes: String, env: &mut Env) -> Result<Expr> {
    let (parsed, _) = parse(&tokenize(lexemes))?;
    let expr = eval(&parsed, env)?;
    Ok(expr)
}

fn get_line() -> String {
    let mut lexemes = String::new();
    io::stdin()
        .read_line(&mut lexemes)
        .expect("Could not read line");
    lexemes.trim().to_owned()
}

pub fn run_repl() -> Result<()> {
    println!("~~~~ Largo ~~~~");
    let mut env = default_env();
    loop {
        print!(">>> ");
        io::stdout().flush()?;
        let line = get_line();
        if line == "quit" {
            break;
        }
        let expr = string_to_exp(line, &mut env)?;
        println!("{}", expr);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_error() {
        assert_eq!(
            "Hello there".to_owned(),
            format!("{}", Error::Reason("Hello there".to_owned()))
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
            Expr::List(vec![
                Expr::Symbol("+".to_owned()),
                Expr::Number(1.0),
                Expr::Number(2.0),
            ])
        );
        assert!(rest.is_empty());
    }

    #[test]
    fn check_parse_atom() {
        assert_eq!(parse_atom("1.0"), Expr::Number(1.0));
        assert_eq!(parse_atom("Hello"), Expr::Symbol("Hello".to_owned()));
        assert_eq!(parse_atom("hi1.0hi"), Expr::Symbol("hi1.0hi".to_owned()));
    }

    #[test]
    fn check_default_env() {
        let Env { data } = default_env();

        let add = *match data.get("+").unwrap() {
            Expr::Func(f) => f,
            _ => panic!("data did not return addition"),
        };
        let sub = *match data.get("-").unwrap() {
            Expr::Func(f) => f,
            _ => panic!("data did not return subtraction"),
        };

        let exps = vec![Expr::Number(1.0), Expr::Number(2.0), Expr::Number(3.0)];

        assert_eq!(add(&exps).unwrap(), Expr::Number(6.0));
        assert_eq!(sub(&exps).unwrap(), Expr::Number(-4.0));
    }

    #[test]
    fn check_eval() {
        let mut env = default_env();

        // Expr::List
        let (exp1, _) = parse(&tokenize("(+ 1 2)".to_owned())).unwrap();
        let (exp2, _) = parse(&tokenize("(+ 1 (+ 2 3 4))".to_owned())).unwrap();
        let (exp3, _) = parse(&tokenize("(- 2 3)".to_owned())).unwrap();
        let (exp4, _) = parse(&tokenize("(- 2 (+ 1 2 3))".to_owned())).unwrap();

        assert_eq!(eval(&exp1, &mut env).unwrap(), Expr::Number(3.0));
        assert_eq!(eval(&exp2, &mut env).unwrap(), Expr::Number(10.0));
        assert_eq!(eval(&exp3, &mut env).unwrap(), Expr::Number(-1.0));
        assert_eq!(eval(&exp4, &mut env).unwrap(), Expr::Number(-4.0));
    }
}
