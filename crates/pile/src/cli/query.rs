//! `pile query`: a Datalog program asked of what the repository declares.

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Value, json};

use query::Answer;
use storage::State;

use super::{Declared, Out, Result, open, os_says, plural, target};
use crate::exit::{Code, Refusal};

pub(super) fn command() -> Command {
    let mut relations = String::new();
    for (name, columns, title) in query::RELATIONS {
        relations.push_str(&format!(
            "  {name}({})\n      {title}\n",
            columns.join(", ")
        ));
    }
    Command::new("query")
        .about("Ask a Datalog program of what the repository declares")
        .long_about(format!(
            "Ask a Datalog program of what the repository declares: its piles, the\n\
             nodes and links their .toml files declare, the types, and where each is\n\
             written. query reads the working copy, from anywhere in it, and writes\n\
             nothing.\n\n\
             The program is one argument, so put it in single quotes, or give the\n\
             file that holds it with --file. A query is a clause that starts with\n\
             ?-, and a program may hold several, with rules of its own before them.\n\
             pile guide query says how a program is written.\n\n\
             A program reads these relations, each written with its columns:\n\
             {relations}\n\
             A node is one value, written as a link writes it: @name for a pile, or\n\
             its :/ path when it has no name of its own, and type:name for any other\n\
             node. In a program, @name is the pile of that name, and one that names\n\
             no pile is refused. An edge is one value too, the first column of edge:\n\
             its source, its type and its target. A type, a property or a value is\n\
             written bare when it is letters, digits and _ after a lowercase letter,\n\
             and in double quotes when it is not: \"relies-on\".\n\n\
             query prints each query as it reads it, then the variables it binds as\n\
             the columns, a row for each answer, sorted, and how many rows there are.\n\
             A query that binds no variable is answered yes or no. A relation the\n\
             program defines is the program's alone, and is gone when query returns.\n\n\
             A program that cannot be read or evaluated is refused, at the line and\n\
             the column of the mistake, and nothing is answered."
        ))
        .override_usage("pile query '<program>'\n       pile query --file <path>")
        .after_help(format!(
            "Examples:\n  \
             pile query '?- node(Node, pile, _).'                    every pile\n  \
             pile query '?- edge(_, Node, \"relies-on\", @shared).'    what relies on @shared\n  \
             pile query '?- prop(Node, labels, service).'            the nodes labelled service\n  \
             pile query --file covers.dl                             a program kept in a file\n\n\
             Refusals:\n  \
             program-refused          1  the program has a mistake: the message says where, and what\n  \
             unreadable-program       2  the file --file names cannot be read\n\
             {}",
            target::REFUSALS
                .lines()
                .filter(|l| {
                    ["no-piles", "unreadable-config"]
                        .iter()
                        .any(|code| l.trim_start().starts_with(code))
                })
                .collect::<Vec<_>>()
                .join("\n")
        ))
        .arg(Arg::new("args").num_args(0..).action(ArgAction::Append))
        .arg(
            Arg::new("file")
                .long("file")
                .value_name("path")
                .help("read the program from this file"),
        )
}

/// The program asked, and the file it was read from when it was.
fn program(m: &ArgMatches) -> Result<(String, Option<String>)> {
    let args: Vec<&String> = m.get_many::<String>("args").into_iter().flatten().collect();
    let file = m.get_one::<String>("file");
    match (args.as_slice(), file) {
        ([one], None) => Ok(((*one).clone(), None)),
        ([], Some(path)) => match std::fs::read_to_string(path) {
            Ok(text) => Ok((text, Some(path.clone()))),
            Err(e) => Err(Refusal::new(
                Code::UnreadableProgram,
                format!("the program in {path} cannot be read: {}", os_says(&e)),
            )),
        },
        ([], None) => Err(Refusal::new(
            Code::Usage,
            "pile query takes a program, or --file and the file that holds one, and was given \
             neither",
        )),
        (_, Some(_)) => Err(Refusal::new(
            Code::Usage,
            "pile query takes a program or --file, and was given both",
        )),
        (many, None) => Err(Refusal::new(
            Code::Usage,
            format!(
                "pile query takes one program, and was given {} arguments: put the program in \
                 single quotes",
                many.len()
            ),
        )),
    }
}

/// A value as a result gives it: an integer as a number, and a symbol as a
/// string.
fn value(v: &query::Value) -> Value {
    match v {
        query::Value::Integer(n) => json!(n),
        query::Value::Symbol(s) => json!(s),
    }
}

/// One answer as text: the query, its columns, its rows, and their count.
fn print(out: &mut Out, a: &Answer) {
    putln!(out, "{}", a.query);
    if a.columns.is_empty() {
        let said = if a.rows.is_empty() { "no" } else { "yes" };
        putln!(out, "   {said}");
        return;
    }
    if a.rows.is_empty() {
        putln!(out, "   no rows");
        return;
    }
    let rows: Vec<Vec<String>> = a
        .rows
        .iter()
        .map(|r| r.iter().map(|v| v.to_string()).collect())
        .collect();
    let width = |i: usize| {
        rows.iter()
            .map(|r| r[i].chars().count())
            .chain([a.columns[i].chars().count()])
            .max()
            .unwrap_or(0)
    };
    let widths: Vec<usize> = (0..a.columns.len()).map(width).collect();
    let line = |out: &mut Out, cells: &[String]| {
        let mut said = String::new();
        for (cell, width) in cells.iter().zip(&widths) {
            said.push_str(&format!("{cell:<width$}  "));
        }
        putln!(out, "   {}", said.trim_end());
    };
    line(out, &a.columns);
    for row in &rows {
        line(out, row);
    }
    putln!(out, "   {}", plural(rows.len(), "row"));
}

/// The result is `answers`, one for each query of the program in the order
/// they are written, each `query`, the query as it was read; `columns`, the
/// variables it binds, in the order they are first written; and `rows`, each
/// a list with a value per column, sorted: a number for an integer, and a
/// string for anything else. A query that binds no variable has no columns,
/// and one empty row when it holds.
///
/// A program that is refused answers as `program-refused`, and the result is
/// `refused`: `file`, the file it was read from or `null`, `line`, `column`
/// and `message`.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    let (program, file) = program(m)?;
    let r = open()?;
    let found = r.storage().declarations(&State::Working)?;
    let s = piles::structure(&found)?;
    if s.is_empty() {
        return Err(target::no_piles(&r));
    }
    let db = Declared::of(&found, &s).db;
    let answers = match query::ask(&db, &program, |name| db.pile_named(name)) {
        Ok(answers) => answers,
        Err(refused) => {
            out.result(json!({"refused": {
                "file": file,
                "line": refused.line,
                "column": refused.column,
                "message": refused.message,
            }}));
            let at = match &file {
                Some(file) => format!("{file}:{}:{}", refused.line, refused.column),
                None => format!("{}:{}", refused.line, refused.column),
            };
            return Err(Refusal::new(
                Code::ProgramRefused,
                format!("the program is refused at {at}: {}", refused.message),
            )
            .next("pile guide query", "says how a program is written")
            .next("pile query --help", "lists the relations a program reads"));
        }
    };
    if answers.is_empty() {
        putln!(
            out,
            "the program asks nothing: a query is a clause that starts with ?-"
        );
        out.next("pile guide query", "says how a query is written");
    }
    for (i, a) in answers.iter().enumerate() {
        if i > 0 {
            putln!(out);
        }
        print(out, a);
    }
    out.result(json!({
        "answers": answers
            .iter()
            .map(|a| json!({
                "query": a.query,
                "columns": a.columns,
                "rows": a.rows.iter().map(|r| r.iter().map(value).collect::<Vec<_>>()).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
    }));
    Ok(())
}
