// ============================================================
// CDOMER - Compilador
// CLI: le um arquivo .cdo, faz lexing -> parsing -> type check
// -> codegen C -> chama o gcc para produzir o binario final.
//
// Uso:
//   cdomer build arquivo.cdo              -> gera ./a.out (ou nome customizado com -o)
//   cdomer build arquivo.cdo -o saida
//   cdomer emit-c arquivo.cdo             -> so imprime o C gerado (nao compila)
//   cdomer run arquivo.cdo                -> compila e executa na hora
// ============================================================

mod lexer;
mod ast;
mod parser;
mod typechecker;
mod codegen;

use std::env;
use std::fs;
use std::process::{Command, exit};

fn print_usage() {
    eprintln!("CDOMER - linguagem C-family com tipagem estatica e inferencia\n");
    eprintln!("Uso:");
    eprintln!("  cdomer build <arquivo.cdo> [-o <saida>]   Compila para um binario nativo");
    eprintln!("  cdomer run <arquivo.cdo>                  Compila e executa imediatamente");
    eprintln!("  cdomer emit-c <arquivo.cdo>                Imprime apenas o codigo C gerado");
}

fn compile_to_c(source: &str, filename: &str) -> Result<String, String> {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("{}: {}", filename, e))?;

    let mut parser = parser::Parser::new(tokens);
    let mut program = parser.parse_program().map_err(|e| format!("{}: {}", filename, e))?;

    let mut checker = typechecker::TypeChecker::new();
    checker.check_program(&mut program).map_err(|e| format!("{}: {}", filename, e))?;

    let mut gen = codegen::CodeGen::new();
    let mut c_code = gen.generate(&program);
    c_code.push_str(&codegen::gen_c_main_wrapper());

    Ok(c_code)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage();
        exit(1);
    }

    let command = args[1].as_str();
    let input_path = &args[2];

    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Erro ao ler '{}': {}", input_path, e);
            exit(1);
        }
    };

    let c_code = match compile_to_c(&source, input_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    match command {
        "emit-c" => {
            println!("{}", c_code);
        }
        "build" | "run" => {
            let mut output_name = "a.out".to_string();
            let mut i = 3;
            while i < args.len() {
                if args[i] == "-o" && i + 1 < args.len() {
                    output_name = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }

            let tmp_c = format!("{}.cdomer.c", input_path);
            if let Err(e) = fs::write(&tmp_c, &c_code) {
                eprintln!("Erro ao escrever C temporario: {}", e);
                exit(1);
            }

            let status = Command::new("gcc")
                .args(["-O2", "-std=c11", "-o", &output_name, &tmp_c, "-lm"])
                .status();

            let _ = fs::remove_file(&tmp_c);

            match status {
                Ok(s) if s.success() => {
                    println!("Compilado com sucesso -> {}", output_name);
                    if command == "run" {
                        let run_status = Command::new(format!("./{}", output_name)).status();
                        match run_status {
                            Ok(rs) => exit(rs.code().unwrap_or(0)),
                            Err(e) => {
                                eprintln!("Erro ao executar '{}': {}", output_name, e);
                                exit(1);
                            }
                        }
                    }
                }
                Ok(s) => {
                    eprintln!("gcc falhou com codigo {:?}", s.code());
                    exit(1);
                }
                Err(e) => {
                    eprintln!("Nao foi possivel executar o gcc: {}", e);
                    eprintln!("Verifique se o gcc esta instalado (no Termux: pkg install clang)");
                    exit(1);
                }
            }
        }
        other => {
            eprintln!("Comando desconhecido: '{}'", other);
            print_usage();
            exit(1);
        }
    }
}
