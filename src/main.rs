use quick_xml::events::Event;
use quick_xml::reader::Reader;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;
use std::process;
use zip::ZipArchive;

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

fn processar_texto(
    texto: &str,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
    frequencias: &mut HashMap<String, usize>,
) {
    let minusculas = texto.to_lowercase();
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

fn contar_txt<R: BufRead>(
    mut leitor: R,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
    frequencias: &mut HashMap<String, usize>,
) -> io::Result<()> {
    let mut linha = String::new();

    loop {
        linha.clear();
        if leitor.read_line(&mut linha)? == 0 {
            break;
        }
        processar_texto(&linha, re, bloqueadas, frequencias);
    }

    Ok(())
}

fn contar_xml_zip<R: Read + io::Seek>(
    arquivo: R,
    entrada_xml: &str,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
    frequencias: &mut HashMap<String, usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut zip = ZipArchive::new(arquivo)?;
    let entrada = zip.by_name(entrada_xml)?;
    let buf_entrada = BufReader::with_capacity(TAMANHO_BUFFER, entrada);

    let mut leitor_xml = Reader::from_reader(buf_entrada);
    leitor_xml.config_mut().trim_text(true);

    let mut buf_xml = Vec::new();

    loop {
        match leitor_xml.read_event_into(&mut buf_xml) {
            Ok(Event::Text(e)) => {
                let texto = std::str::from_utf8(e.as_ref())?;
                processar_texto(texto, re, bloqueadas, frequencias);
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(Box::new(err)),
            _ => {}
        }
        buf_xml.clear();
    }

    Ok(())
}

fn contar_arquivo(
    caminho: &Path,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
) -> Result<HashMap<String, usize>, Box<dyn std::error::Error>> {
    let mut frequencias: HashMap<String, usize> = HashMap::new();

    let ext = caminho
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "docx" => {
            let arquivo = File::open(caminho)?;
            contar_xml_zip(arquivo, "word/document.xml", re, bloqueadas, &mut frequencias)?;
        }
        "odt" => {
            let arquivo = File::open(caminho)?;
            contar_xml_zip(arquivo, "content.xml", re, bloqueadas, &mut frequencias)?;
        }
        _ => {
            let arquivo = File::open(caminho)?;
            let leitor = BufReader::with_capacity(TAMANHO_BUFFER, arquivo);
            contar_txt(leitor, re, bloqueadas, &mut frequencias)?;
        }
    }

    Ok(frequencias)
}

fn main() {
    let arg0 = env::args().next().unwrap_or_default();
    let programa = Path::new(&arg0)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("contador_palavras");

    let caminho_str = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Uso: {} <arquivo.txt|arquivo.docx|arquivo.odt>", programa);
        process::exit(1);
    });

    let caminho = Path::new(&caminho_str);
    let re = Regex::new(r"[a-zà-ú]{3,}").expect("Regex inválido");
    let bloqueadas = stop_words();

    let frequencias = contar_arquivo(caminho, &re, &bloqueadas).unwrap_or_else(|err| {
        eprintln!("Erro ao processar '{}': {}", caminho.display(), err);
        process::exit(1);
    });

    let mut ranking: Vec<(String, usize)> = frequencias.into_iter().collect();
    ranking.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    for (palavra, contagem) in ranking.into_iter().take(30) {
        println!("{:7} {}", contagem, palavra);
    }
}