# CDOMER

Linguagem de programação **C-family**, com **tipagem estática + inferência de tipos**,
cujo compilador (escrito em **Rust**) transpila o código CDOMER para **C11** e
em seguida invoca o `gcc`/`clang` para gerar um binário nativo.

```
CDOMER (.cdo) -> [lexer] -> [parser] -> [type checker] -> [codegen C] -> gcc -> binário nativo
```

## Por que CDOMER?

- Sintaxe familiar pra quem programa em C/Java/Rust/TS.
- Tipagem estática (erros pegos em tempo de compilação), mas com `let` inferindo
  o tipo quando você não quer escrever (`let x = 10;` já sabe que é `int`).
- Compila pra C de verdade — roda em qualquer lugar que tenha `gcc`/`clang`,
  inclusive Termux no Android, sem dependências externas em runtime.

---

## Instalação (Debian/Apt) 

```bash
sudo apt install rust clang
cd cdomer
cargo build --release
# binário fica em target/release/cdomer
cp target/release/cdomer $PREFIX/bin/cdomer   # opcional: deixa global
```

Em qualquer outro Linux com Rust + gcc instalados, o processo é o mesmo
(`cargo build --release`).

## Uso

```bash
cdomer build examples/hello.cdo -o hello   # compila para o binário ./hello
./hello

cdomer run examples/fib.cdo                # compila e roda na hora

cdomer emit-c examples/hello.cdo           # so mostra o C gerado, sem compilar
```

---

## Tour pela linguagem

### Variáveis e inferência de tipo

```cdomer
fn main() {
    let nome: string = "Lil";   // tipo explícito
    let idade = 25;             // inferido como int
    let pi = 3.14;               // inferido como float
    let ligado = true;          // inferido como bool
}
```

### Tipos primitivos

| CDOMER   | C gerado |
|----------|----------|
| `int`    | `long`   |
| `float`  | `double` |
| `bool`   | `bool`   |
| `string` | `char*`  |
| `void`   | `void`   |
| `T[]`    | struct `cdomer_arr_T` (data + len) |

### Funções

```cdomer
fn soma(a: int, b: int) -> int {
    return a + b;
}

fn saudacao(nome: string) {   // sem -> tipo == void
    print("Oi,", nome);
}
```

### Controle de fluxo

```cdomer
if (x > 10) {
    print("grande");
} else if (x > 0) {
    print("positivo");
} else {
    print("nao positivo");
}

while (x < 100) {
    x += 1;
}

for (let i = 0; i < 10; i += 1) {
    print(i);
}
```

`break;` e `continue;` funcionam dentro de `while`/`for`.

### Structs

```cdomer
struct Ponto {
    x: int,
    y: int
}

fn main() {
    let p = Ponto { x: 3, y: 4 };
    print(p.x, p.y);
}
```

### Arrays

```cdomer
fn main() {
    let nums = [1, 2, 3, 4, 5];
    print(nums.len);     // 5
    print(nums[0]);       // 1
}
```

### print

`print(a, b, c, ...)` aceita qualquer mistura de `int`, `float`, `bool`,
`string` e imprime tudo separado por espaço, com `\n` no final. É resolvido
em tempo de compilação (vira um `printf` com o formato certo pra cada
argumento — sem overhead de tipagem dinâmica).

### Operadores

| Categoria   | Operadores |
|-------------|------------|
| Aritméticos | `+ - * / %` |
| Atribuição  | `= += -= *= /=` |
| Comparação  | `== != < > <= >=` |
| Lógicos     | `&& \|\| !` |
| Comentários | `// linha` e `/* bloco */` |

---

## Regras de tipagem

- `let` sem anotação infere o tipo a partir do valor.
- `let` com anotação verifica compatibilidade (com promoção `int -> float`
  permitida).
- Toda função precisa que os tipos de retorno batam exatamente (ou `int`
  promovido a `float`).
- Não há conversões implícitas entre `string`/`bool`/`int`/`float` fora da
  promoção numérica.
- Todo programa precisa de uma `fn main()`.

## Erros

Erros de léxico, sintaxe e tipo são reportados com linha/coluna, por exemplo:

```
Erro de tipo [linha 2]: tipo declarado 'int' nao bate com o tipo do valor 'string' na variavel 'x'
```

---

## Estrutura do projeto

```
cdomer/
├── Cargo.toml
├── src/
│   ├── main.rs         CLI: build / run / emit-c
│   ├── lexer.rs         tokenizador
│   ├── ast.rs            definição da AST
│   ├── parser.rs         parser de descida recursiva
│   ├── typechecker.rs    checagem + inferência de tipos
│   └── codegen.rs        transpila AST -> C11
├── examples/
│   ├── hello.cdo
│   ├── fib.cdo           (recursão)
│   ├── structs.cdo       (structs)
│   └── arrays_loops.cdo  (arrays + while)
└── tests/
    └── run_examples.sh   roda todos os exemplos e confere a saída
```

## Limitações conhecidas (v0.1)

- Sem genéricos.
- Sem ponteiros expostos na linguagem (uso interno só no codegen).
- Sem módulos/imports — um programa é um arquivo só.
- Arrays são de tamanho fixo definido no literal (`[1,2,3]`); não há
  `push`/resize ainda.
- `print` não tem `%`-style formatting manual, é posicional automático.

Tudo isso dá pra estender depois — a arquitetura em camadas (lexer → parser →
checker → codegen) foi pensada exatamente pra isso ser incremental.
