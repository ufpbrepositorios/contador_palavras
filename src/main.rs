use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process;

const TAMANHO_BUFFER: usize = 64 * 1024;

fn stop_words() -> HashSet<&'static str> {
    [
        "que", "para", "com", "não", "uma", "por", "mais", "dos", "como", "mas",
        "foi", "ele", "das", "tem", "seu", "sua", "ou", "ser", "quando", "muito",
        "nos", "já", "está", "eu", "também", "só", "pelo", "pela", "até", "isso",
        "ela", "entre", "depois", "sem", "mesmo", "aos", "seus", "quem", "nas",
        "me", "esse", "eles", "você", "essa", "num", "nem", "suas", "meu", "às",
        "minha", "numa", "pelos", "elas", "qual", "nós", "lhe", "deles", "essas",
        "esses", "pelas", "este", "dele", "tu", "te", "vocês", "vos", "lhes",
        "meus", "minhas", "teu", "tua", "teus", "tuas", "nosso", "nossa",
        "nossos", "nossas", "dela", "delas", "esta", "estes", "estas", "aquele",
        "aquela", "aqueles", "aquelas", "isto", "aquilo",
    ]
    .into_iter()
    .collect()
}

fn contar_palavras<R: BufRead>(
    leitor: R,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
) -> io::Result<HashMap<String, usize>> {
    let mut leitor = leitor;
    let mut linha = String::new();
    let mut frequencias: HashMap<String, usize> = HashMap::new();

    loop {
        linha.clear();
        if leitor.read_line(&mut linha)? == 0 {
            break;
        }

        let minusculas = linha.to_lowercase();
        for m in re.find_iter(&minusculas) {
            let palavra = m.as_str();
            if !bloqueadas.contains(palavra) {
                if let Some(contagem) = frequencias.get_mut(palavra) {
                    *contagem += 1;
                } else {
                    frequencias.insert(palavra.to_string(), 1);
                }
            }
        }
    }

    Ok(frequencias)
}

fn main() {
    let programa = env::args()
        .next()
        .unwrap_or_else(|| "contador_palavras".to_string());

    let caminho = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Uso: {} <caminho/do/arquivo.txt>", programa);
        process::exit(1);
    });

    let arquivo = File::open(&caminho).unwrap_or_else(|err| {
        eprintln!("Erro ao abrir '{}': {}", caminho, err);
        process::exit(1);
    });

    let leitor = BufReader::with_capacity(TAMANHO_BUFFER, arquivo);
    let re = Regex::new(r"[a-zà-ú]{3,}").expect("Regex inválido");
    let bloqueadas = stop_words();

    let frequencias = contar_palavras(leitor, &re, &bloqueadas).unwrap_or_else(|err| {
        eprintln!("Erro ao ler '{}': {}", caminho, err);
        process::exit(1);
    });

    let mut ranking: Vec<(String, usize)> = frequencias.into_iter().collect();
    ranking.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    for (palavra, contagem) in ranking.into_iter().take(30) {
        println!("{:7} {}", contagem, palavra);
    }
}

