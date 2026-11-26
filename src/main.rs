use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;
use std::process;
use zip::ZipArchive;

const TAMANHO_BUFFER: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatoArquivo {
    Txt,
    Pdf,
    Odt,
    Docx,
}

fn stop_words() -> HashSet<&'static str> {
    [
        // Artigos e contrações
        "uns", "umas", "aos", "das", "dos", "nas", "nos", "pelas", "pelos",
        // Preposições e conjunções
        "até", "com", "como", "contra", "desde", "entre", "mas", "nem", "para",
        "perante", "por", "que", "sem", "sob", "sobre", "trás",
        // Pronomes pessoais, possessivos, demonstrativos e indefinidos
        "ela", "elas", "ele", "eles", "lhe", "lhes", "meu", "meus", "minha",
        "minhas", "nós", "nosso", "nossa", "nossos", "nossas", "seu", "seus",
        "sua", "suas", "teu", "teus", "tua", "tuas", "você", "vocês", "vós",
        "vosso", "vossa", "vossos", "vossas", "dela", "delas", "dele", "deles",
        "esta", "estas", "este", "estes", "isto", "essa", "essas", "esse",
        "esses", "isso", "aquela", "aquelas", "aquele", "aqueles", "aquilo",
        "algo", "alguém", "algum", "alguma", "alguns", "algumas", "cada",
        "certo", "certa", "certos", "certas", "muito", "muita", "muitos",
        "muitas", "nada", "ninguém", "nenhum", "nenhuma", "outro", "outra",
        "outros", "outras", "pouco", "pouca", "poucos", "poucas", "qual",
        "quais", "qualquer", "quaisquer", "quanto", "quanta", "quantos",
        "quantas", "quem", "tanto", "tanta", "tantos", "tantas", "todo",
        "toda", "todos", "todas", "tudo", "vários", "várias",
        // Advérbios, verbos auxiliares e termos frequentes
        "ainda", "antes", "aqui", "assim", "bem", "depois", "então", "está",
        "estão", "estava", "estavam", "foi", "foram", "havia", "hoje", "logo",
        "mais", "menos", "mesmo", "mesma", "mesmos", "mesmas", "não", "num",
        "numa", "nunca", "onde", "ontem", "pode", "podem", "pois", "quando",
        "sempre", "ser", "será", "serão", "sido", "sim", "somos", "são",
        "também", "tão", "tem", "têm", "tinha", "tinham", "vai", "vão",
    ]
    .into_iter()
    .collect()
}

/// Converte `origem` para minúsculas escrevendo diretamente em `destino`
/// para evitar realocações repetidas.
fn para_minusculas_no_buffer(origem: &str, destino: &mut String) {
    destino.clear();
    for ch in origem.chars() {
        for lower in ch.to_lowercase() {
            destino.push(lower);
        }
    }
}

fn processar_texto(
    texto: &str,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
    frequencias: &mut HashMap<String, usize>,
) {
    for m in re.find_iter(texto) {
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

fn contar_palavras_txt<R: BufRead>(
    mut leitor: R,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
) -> io::Result<HashMap<String, usize>> {
    let mut linha = String::new();
    let mut linha_minuscula = String::new();
    let mut frequencias: HashMap<String, usize> = HashMap::new();

    loop {
        linha.clear();
        if leitor.read_line(&mut linha)? == 0 {
            break;
        }

        para_minusculas_no_buffer(&linha, &mut linha_minuscula);
        processar_texto(&linha_minuscula, re, bloqueadas, &mut frequencias);
    }

    Ok(frequencias)
}

fn contar_palavras_pdf(
    caminho: &Path,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
) -> Result<HashMap<String, usize>, String> {
    let texto_extraido = pdf_extract::extract_text(caminho)
        .map_err(|err| format!("Falha ao extrair texto do PDF: {}", err))?;

    let mut texto_minusculo = String::new();
    para_minusculas_no_buffer(&texto_extraido, &mut texto_minusculo);

    let mut frequencias: HashMap<String, usize> = HashMap::new();
    processar_texto(&texto_minusculo, re, bloqueadas, &mut frequencias);

    Ok(frequencias)
}

fn extrair_texto_xml_zip(
    caminho: &Path,
    entrada_xml: &str,
    tags_quebra_bloco: &[&[u8]],
) -> Result<String, String> {
    let arquivo = File::open(caminho)
        .map_err(|err| format!("Erro ao abrir '{}': {}", caminho.display(), err))?;
    let mut zip = ZipArchive::new(arquivo)
        .map_err(|err| format!("Arquivo ZIP inválido em '{}': {}", caminho.display(), err))?;

    let mut entrada = zip
        .by_name(entrada_xml)
        .map_err(|err| format!("Entrada '{}' não encontrada no arquivo: {}", entrada_xml, err))?;

    let mut xml_bytes = Vec::new();
    entrada
        .read_to_end(&mut xml_bytes)
        .map_err(|err| format!("Erro ao ler '{}': {}", entrada_xml, err))?;

    let mut leitor = Reader::from_reader(&xml_bytes[..]);
    leitor.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut saida = String::new();

    loop {
        match leitor.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let nome = e.local_name();
                if tags_quebra_bloco.iter().any(|tag| nome.as_ref() == *tag) && !saida.ends_with(' ') {
                    saida.push(' ');
                }
            }
            Ok(Event::Text(e)) => {
                let texto = e
                    .unescape()
                    .map_err(|err| format!("Erro ao decodificar entidades XML: {}", err))?;
                saida.push_str(&texto);
            }
            Ok(Event::End(e)) => {
                let nome = e.local_name();
                if tags_quebra_bloco.iter().any(|tag| nome.as_ref() == *tag) && !saida.ends_with(' ') {
                    saida.push(' ');
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(format!("Erro ao analisar XML '{}': {}", entrada_xml, err)),
            _ => {}
        }
        buf.clear();
    }

    Ok(saida)
}

fn contar_palavras_odt(
    caminho: &Path,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
) -> Result<HashMap<String, usize>, String> {
    let tags_bloco: &[&[u8]] = &[b"p", b"h", b"tab", b"line-break", b"s"];
    let texto_extraido = extrair_texto_xml_zip(caminho, "content.xml", tags_bloco)?;

    let mut texto_minusculo = String::new();
    para_minusculas_no_buffer(&texto_extraido, &mut texto_minusculo);

    let mut frequencias: HashMap<String, usize> = HashMap::new();
    processar_texto(&texto_minusculo, re, bloqueadas, &mut frequencias);

    Ok(frequencias)
}

fn contar_palavras_docx(
    caminho: &Path,
    re: &Regex,
    bloqueadas: &HashSet<&str>,
) -> Result<HashMap<String, usize>, String> {
    let tags_bloco: &[&[u8]] = &[b"p", b"br", b"tab"];
    let texto_extraido = extrair_texto_xml_zip(caminho, "word/document.xml", tags_bloco)?;

    let mut texto_minusculo = String::new();
    para_minusculas_no_buffer(&texto_extraido, &mut texto_minusculo);

    let mut frequencias: HashMap<String, usize> = HashMap::new();
    processar_texto(&texto_minusculo, re, bloqueadas, &mut frequencias);

    Ok(frequencias)
}

fn detectar_formato(caminho: &Path) -> FormatoArquivo {
    match caminho
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("pdf") => FormatoArquivo::Pdf,
        Some("odt") => FormatoArquivo::Odt,
        Some("docx") => FormatoArquivo::Docx,
        _ => FormatoArquivo::Txt,
    }
}

fn main() {
    let programa = env::args()
        .next()
        .unwrap_or_else(|| "wordcounter".to_string());

    let caminho_str = env::args().nth(1).unwrap_or_else(|| {
        eprintln!(
            "Uso: {} <caminho/do/arquivo.(txt|pdf|odt|docx)>

Reportar erros para <douglas@sti.ufpb.br>.
<https://github.com/ufpbrepositorios/contador_palavras>",
            programa
        );
        process::exit(1);
    });

    let caminho = Path::new(&caminho_str);
    let re = Regex::new(r"\p{L}{3,}").expect("Regex inválido");
    let bloqueadas = stop_words();

    let frequencias = match detectar_formato(caminho) {
        FormatoArquivo::Pdf => contar_palavras_pdf(caminho, &re, &bloqueadas).unwrap_or_else(|err| {
            eprintln!("Erro ao processar PDF '{}': {}", caminho_str, err);
            process::exit(1);
        }),
        FormatoArquivo::Odt => contar_palavras_odt(caminho, &re, &bloqueadas).unwrap_or_else(|err| {
            eprintln!("Erro ao processar ODT '{}': {}", caminho_str, err);
            process::exit(1);
        }),
        FormatoArquivo::Docx => {
            contar_palavras_docx(caminho, &re, &bloqueadas).unwrap_or_else(|err| {
                eprintln!("Erro ao processar DOCX '{}': {}", caminho_str, err);
                process::exit(1);
            })
        }
        FormatoArquivo::Txt => {
            let arquivo = File::open(caminho).unwrap_or_else(|err| {
                eprintln!("Erro ao abrir '{}': {}", caminho_str, err);
                process::exit(1);
            });
            let leitor = BufReader::with_capacity(TAMANHO_BUFFER, arquivo);
            contar_palavras_txt(leitor, &re, &bloqueadas).unwrap_or_else(|err| {
                eprintln!("Erro ao ler '{}': {}", caminho_str, err);
                process::exit(1);
            })
        }
    };

    let mut ranking: Vec<(String, usize)> = frequencias.into_iter().collect();
    ranking.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    for (palavra, contagem) in ranking.into_iter().take(30) {
        println!("{:7} {}", contagem, palavra);
    }
}