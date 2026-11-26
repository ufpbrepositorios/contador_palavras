use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;
use zip::ZipArchive;

const TAMANHO_BUFFER: usize = 64 * 1024;
const TOP_RANKING: usize = 30;
const COLUNAS_STOP_WORDS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatoArquivo {
    Txt,
    Pdf,
    Odt,
    Docx,
}

impl FormatoArquivo {
    fn rotulo(self) -> &'static str {
        match self {
            FormatoArquivo::Txt => "TXT",
            FormatoArquivo::Pdf => "PDF",
            FormatoArquivo::Odt => "ODT",
            FormatoArquivo::Docx => "DOCX",
        }
    }
}

#[derive(Debug)]
struct Opcoes {
    arquivo: Option<PathBuf>,
    verbose: bool,
    ajuda: bool,
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

fn imprimir_stop_words(bloqueadas: &HashSet<&str>) {
    let mut palavras: Vec<&&str> = bloqueadas.iter().collect();
    palavras.sort_unstable();

    println!("\nStop words ignoradas ({}):", palavras.len());
    for (idx, palavra) in palavras.iter().enumerate() {
        print!("{:<12}", palavra);
        if (idx + 1) % COLUNAS_STOP_WORDS == 0 {
            println!();
        }
    }
    if palavras.len() % COLUNAS_STOP_WORDS != 0 {
        println!();
    }
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

fn imprimir_ajuda(programa: &str, bloqueadas: &HashSet<&str>) {
    println!(
        "Uso: {0} -f <arquivo.(txt|pdf|odt|docx)> [-v]
     {0} -h

Contador de frequência de palavras (ignora pronomes, artigos e outras palavras com menos de 3 letras).

Opções:
  -f <arquivo>   Caminho do arquivo de entrada a ser processado
  -v, --verbose  Exibe informações detalhadas do processamento em stderr
  -h, --help     Exibe esta mensagem de ajuda, a lista de stop words e sai

Exemplos:
  {0} -f arquivo.txt
  {0} -f relatorio.pdf -v
  {0} -h

Reportar erros para <douglas@sti.ufpb.br>.
<https://github.com/ufpbrepositorios/contador_palavras>",
        programa
    );

    imprimir_stop_words(bloqueadas);
}

fn parsear_argumentos<I>(args: I) -> Result<Opcoes, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let _programa = iter.next();

    let mut opcoes = Opcoes {
        arquivo: None,
        verbose: false,
        ajuda: false,
    };

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                opcoes.ajuda = true;
            }
            "-v" | "--verbose" => {
                opcoes.verbose = true;
            }
            "-f" => {
                let valor = iter
                    .next()
                    .ok_or_else(|| "A opção '-f' exige um caminho de arquivo.".to_string())?;
                if valor.trim().is_empty() {
                    return Err("O caminho informado para '-f' está vazio.".to_string());
                }
                opcoes.arquivo = Some(PathBuf::from(valor));
            }
            outro if outro.starts_with("-f") && outro.len() > 2 => {
                let valor = &outro[2..];
                opcoes.arquivo = Some(PathBuf::from(valor));
            }
            outro => {
                return Err(format!(
                    "Parâmetro desconhecido '{}'. Use '-h' para ver as opções disponíveis.",
                    outro
                ));
            }
        }
    }

    Ok(opcoes)
}

fn main() {
    let raw_args: Vec<String> = env::args().collect();
    let programa = raw_args
        .first()
        .cloned()
        .unwrap_or_else(|| "wordcounter".to_string());

    if raw_args.len() <= 1 {
        eprintln!(
            "Erro: nenhum parâmetro informado. Use '{} -f <arquivo>' ou '{} -h' para ajuda.",
            programa, programa
        );
        process::exit(1);
    }

    let opcoes = parsear_argumentos(raw_args).unwrap_or_else(|err| {
        eprintln!("Erro nos parâmetros: {}", err);
        process::exit(1);
    });

    let bloqueadas = stop_words();

    if opcoes.ajuda {
        imprimir_ajuda(&programa, &bloqueadas);
        return;
    }

    let caminho = opcoes.arquivo.unwrap_or_else(|| {
        eprintln!(
            "Erro: o parâmetro '-f <arquivo>' é obrigatório. Use '{} -h' para ajuda.",
            programa
        );
        process::exit(1);
    });

    let caminho_str = caminho.display().to_string();
    let formato = detectar_formato(&caminho);
    let re = Regex::new(r"\p{L}{3,}").expect("Regex inválido");

    if opcoes.verbose {
        eprintln!("[verbose] Arquivo: {}", caminho_str);
        eprintln!("[verbose] Formato detectado: {}", formato.rotulo());
        eprintln!("[verbose] Stop words carregadas: {}", bloqueadas.len());
    }

    let frequencias = match formato {
        FormatoArquivo::Pdf => contar_palavras_pdf(&caminho, &re, &bloqueadas).unwrap_or_else(|err| {
            eprintln!("Erro ao processar PDF '{}': {}", caminho_str, err);
            process::exit(1);
        }),
        FormatoArquivo::Odt => contar_palavras_odt(&caminho, &re, &bloqueadas).unwrap_or_else(|err| {
            eprintln!("Erro ao processar ODT '{}': {}", caminho_str, err);
            process::exit(1);
        }),
        FormatoArquivo::Docx => {
            contar_palavras_docx(&caminho, &re, &bloqueadas).unwrap_or_else(|err| {
                eprintln!("Erro ao processar DOCX '{}': {}", caminho_str, err);
                process::exit(1);
            })
        }
        FormatoArquivo::Txt => {
            let arquivo = File::open(&caminho).unwrap_or_else(|err| {
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

    if opcoes.verbose {
        let total_ocorrencias: usize = frequencias.values().sum();
        eprintln!(
            "[verbose] Palavras únicas (após filtros): {} | Total de ocorrências: {}",
            frequencias.len(),
            total_ocorrencias
        );
        eprintln!("[verbose] Exibindo até as {} palavras mais frequentes:", TOP_RANKING);
    }

    let mut ranking: Vec<(String, usize)> = frequencias.into_iter().collect();
    ranking.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    for (palavra, contagem) in ranking.into_iter().take(TOP_RANKING) {
        println!("{:7} {}", contagem, palavra);
    }
}
