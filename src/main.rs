/// file-organizer — Organizador paralelo de arquivos por extensão
///
/// Percorre recursivamente um diretório de origem, coleta todos os arquivos,
/// agrupa-os por extensão e os move (ou copia) para subpastas nomeadas pela
/// extensão dentro de um diretório de destino.
///
/// Uso típico:
///   file-organizer --source /mnt/hd/pasta --dest /mnt/hd
///
/// Funcionalidades:
///   - Processamento paralelo via Rayon (usa todos os núcleos disponíveis)
///   - Barra de progresso em tempo real
///   - Modo --dry-run para simular sem mover nada
///   - Modo --copy para copiar em vez de mover
///   - Renomeia automaticamente arquivos duplicados (não sobrescreve)
///   - Relatório final com estatísticas detalhadas
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use uuid::Uuid;

// ─── Estrutura de argumentos CLI ──────────────────────────────────────────────

/// Organizador de arquivos por extensão.
/// Move (ou copia) arquivos de subpastas para pastas organizadas por extensão.
#[derive(Parser, Debug)]
#[command(
    name = "file-organizer",
    version = "1.0.0",
    about = "Organiza arquivos por extensão de forma rápida e paralela",
    long_about = "Percorre recursivamente o diretório SOURCE, agrupa todos os \
                  arquivos por extensão e os move para DEST/<extensão>/.\n\n\
                  Exemplo:\n  file-organizer --source /mnt/hd/fotos --dest /mnt/hd\n\n\
                  Resultado:\n  /mnt/hd/jpg/   ← todos os .jpg\n  \
                  /mnt/hd/mp4/   ← todos os .mp4\n  /mnt/hd/sem-extensao/ ← sem extensão"
)]
struct Args {
    /// Diretório de origem — será percorrido recursivamente
    #[arg(short, long)]
    source: PathBuf,

    /// Diretório de destino — receberá as pastas por extensão
    #[arg(short, long)]
    dest: PathBuf,

    /// Apenas simula as operações, sem mover/copiar nada
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Copia os arquivos em vez de mover (move é o padrão)
    #[arg(short, long, default_value_t = false)]
    copy: bool,

    /// Número de threads paralelas (padrão: todos os núcleos disponíveis)
    #[arg(short, long)]
    threads: Option<usize>,

    /// Não exibe barra de progresso (útil para scripts/logs)
    #[arg(long, default_value_t = false)]
    quiet: bool,

    /// Inclui arquivos ocultos (que começam com ponto)
    #[arg(long, default_value_t = false)]
    include_hidden: bool,
}

// ─── Representação de um arquivo encontrado ───────────────────────────────────

/// Metadados de um arquivo descoberto durante a varredura.
#[derive(Debug, Clone)]
struct FileEntry {
    /// Caminho absoluto do arquivo original
    path: PathBuf,
    /// Extensão normalizada (minúsculas), ou "sem-extensao" se ausente
    extension: String,
    /// Tamanho em bytes
    size: u64,
}

// ─── Resultado de uma operação de mover/copiar ────────────────────────────────

/// Resultado do processamento de um único arquivo.
enum OpResult {
    Ok { bytes: u64 },
    Skipped { reason: String },
    Err { path: PathBuf, reason: String },
}

// ─── Ponto de entrada ─────────────────────────────────────────────────────────

fn main() {
    let args = Args::parse();

    // Configura o pool de threads do Rayon se o usuário especificou um número
    if let Some(n) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .expect("Falha ao configurar pool de threads");
    }

    // Exibe cabeçalho
    if !args.quiet {
        println!();
        println!("{}", "╔══════════════════════════════════════╗".cyan());
        println!("{}", "║       FILE ORGANIZER  v1.0.0         ║".cyan());
        println!("{}", "╚══════════════════════════════════════╝".cyan());
        println!();
        println!("  {} {}", "Origem:".bold(), args.source.display());
        println!("  {} {}", "Destino:".bold(), args.dest.display());
        println!(
            "  {} {}",
            "Modo:".bold(),
            if args.dry_run {
                "DRY-RUN (simulação)".yellow().to_string()
            } else if args.copy {
                "CÓPIA".blue().to_string()
            } else {
                "MOVER".green().to_string()
            }
        );
        println!();
    }

    // Valida diretório de origem
    if !args.source.exists() {
        eprintln!(
            "{} O diretório de origem não existe: {}",
            "ERRO:".red().bold(),
            args.source.display()
        );
        std::process::exit(1);
    }

    if !args.source.is_dir() {
        eprintln!(
            "{} O caminho de origem não é um diretório: {}",
            "ERRO:".red().bold(),
            args.source.display()
        );
        std::process::exit(1);
    }

    // Etapa 1: Varredura — coleta todos os arquivos recursivamente
    if !args.quiet {
        print!("{}", "  [1/3] Varrendo arquivos...".dimmed());
    }

    let files = scan_files(&args.source, args.include_hidden);

    if !args.quiet {
        println!(
            "\r  {}  {} arquivos encontrados",
            "[1/3] Varredura concluída.".green(),
            files.len().to_string().bold()
        );
    }

    if files.is_empty() {
        println!("{}", "  Nenhum arquivo encontrado. Encerrando.".yellow());
        return;
    }

    // Etapa 2: Agrupamento por extensão (apenas para exibir estatísticas)
    let mut por_extensao: HashMap<&str, usize> = HashMap::new();
    for f in &files {
        *por_extensao.entry(f.extension.as_str()).or_insert(0) += 1;
    }

    if !args.quiet {
        println!(
            "  {}  {} extensões distintas encontradas",
            "[2/3] Agrupamento concluído.".green(),
            por_extensao.len().to_string().bold()
        );
    }

    // Etapa 3: Processamento paralelo — mover/copiar arquivos
    if !args.quiet {
        println!("  {}", "[3/3] Processando arquivos...".dimmed());
    }

    let (ok, skipped, errors, total_bytes) =
        process_files(&files, &args.dest, &args, args.quiet);

    // ─── Relatório final ─────────────────────────────────────────────────────

    println!();
    println!("{}", "══════════════ RELATÓRIO FINAL ══════════════".cyan().bold());
    println!(
        "  {} {}",
        "✔  Processados com sucesso:".green().bold(),
        ok.to_string().bold()
    );
    if skipped > 0 {
        println!(
            "  {} {}",
            "⊘  Ignorados:".yellow().bold(),
            skipped.to_string().bold()
        );
    }
    if errors > 0 {
        println!(
            "  {} {}",
            "✘  Erros:".red().bold(),
            errors.to_string().bold()
        );
    }
    println!(
        "  {} {}",
        "   Volume transferido:".bold(),
        format_bytes(total_bytes)
    );

    if args.dry_run {
        println!();
        println!(
            "  {}",
            "Modo DRY-RUN: nenhum arquivo foi movido/copiado.".yellow()
        );
    }
    println!();
}

// ─── Varredura recursiva ──────────────────────────────────────────────────────

/// Percorre `dir` recursivamente e retorna todos os arquivos encontrados.
///
/// Usa `WalkDir` emulado via `std::fs::read_dir` recursivo para evitar
/// dependências extras. Para desempenho máximo em varreduras grandes,
/// poderíamos usar `jwalk` (paralelo), mas `std::fs` é suficiente aqui
/// porque o gargalo real é o I/O do disco, não a CPU.
fn scan_files(dir: &Path, include_hidden: bool) -> Vec<FileEntry> {
    let mut result = Vec::new();
    scan_recursive(dir, include_hidden, &mut result);
    result
}

/// Implementação recursiva da varredura de diretórios.
fn scan_recursive(dir: &Path, include_hidden: bool, out: &mut Vec<FileEntry>) {
    // Lê o conteúdo do diretório — ignora silenciosamente erros de permissão
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Ignora arquivos/pastas ocultos, a menos que --include-hidden seja passado
        if !include_hidden {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
        }

        if path.is_dir() {
            // Descende recursivamente
            scan_recursive(&path, include_hidden, out);
        } else if path.is_file() {
            // Determina a extensão (normalizada para minúsculas)
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_else(|| "sem-extensao".to_string());

            // Obtém tamanho do arquivo (0 se falhar)
            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            out.push(FileEntry {
                path,
                extension,
                size,
            });
        }
    }
}

// ─── Processamento paralelo ───────────────────────────────────────────────────

/// Processa todos os arquivos em paralelo usando Rayon.
///
/// Retorna `(ok, skipped, errors, total_bytes)`.
fn process_files(
    files: &[FileEntry],
    dest: &Path,
    args: &Args,
    quiet: bool,
) -> (u64, u64, u64, u64) {
    // Contadores atômicos — seguros para acesso simultâneo entre threads
    let count_ok = Arc::new(AtomicU64::new(0));
    let count_skipped = Arc::new(AtomicU64::new(0));
    let count_err = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));

    // Lista de erros acumulados (protegida por Mutex)
    let errors_log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Configura barra de progresso
    let pb = if !quiet {
        let bar = ProgressBar::new(files.len() as u64);
        bar.set_style(
            ProgressStyle::with_template(
                "  [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}",
            )
            .unwrap()
            .progress_chars("█▓░"),
        );
        Some(Arc::new(bar))
    } else {
        None
    };

    // ── Processamento paralelo com Rayon ──────────────────────────────────────
    // `par_iter()` divide automaticamente o slice entre as threads disponíveis.
    // Cada thread processa um subconjunto de arquivos de forma independente.
    files.par_iter().for_each(|file| {
        let result = process_single(file, dest, args);

        match result {
            OpResult::Ok { bytes } => {
                count_ok.fetch_add(1, Ordering::Relaxed);
                total_bytes.fetch_add(bytes, Ordering::Relaxed);
            }
            OpResult::Skipped { .. } => {
                count_skipped.fetch_add(1, Ordering::Relaxed);
            }
            OpResult::Err { path, reason } => {
                count_err.fetch_add(1, Ordering::Relaxed);
                if let Ok(mut log) = errors_log.lock() {
                    log.push(format!("  {} {}: {}", "✘".red(), path.display(), reason));
                }
            }
        }

        // Avança a barra de progresso
        if let Some(ref bar) = pb {
            bar.inc(1);
        }
    });

    // Finaliza a barra de progresso
    if let Some(bar) = pb {
        bar.finish_and_clear();
    }

    // Exibe erros coletados
    let log = errors_log.lock().unwrap();
    if !log.is_empty() {
        println!();
        println!("{}", "  Erros encontrados:".red().bold());
        for msg in log.iter().take(20) {
            // exibe no máximo 20 erros
            println!("{}", msg);
        }
        if log.len() > 20 {
            println!("  ... e mais {} erros.", log.len() - 20);
        }
    }

    (
        count_ok.load(Ordering::Relaxed),
        count_skipped.load(Ordering::Relaxed),
        count_err.load(Ordering::Relaxed),
        total_bytes.load(Ordering::Relaxed),
    )
}

/// Processa um único arquivo: decide o destino e executa mover/copiar.
fn process_single(file: &FileEntry, dest_root: &Path, args: &Args) -> OpResult {
    // Pasta de destino: dest_root/<extensão>/
    let dest_dir = dest_root.join(&file.extension);

    // Cria a pasta de destino se não existir (no modo dry-run, pula)
    if !args.dry_run {
        if let Err(e) = fs::create_dir_all(&dest_dir) {
            return OpResult::Err {
                path: file.path.clone(),
                reason: format!("Não foi possível criar diretório {:?}: {}", dest_dir, e),
            };
        }
    }

    // Nome do arquivo original
    let file_name = match file.path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_string(),
        None => {
            return OpResult::Err {
                path: file.path.clone(),
                reason: "Nome de arquivo inválido (não-UTF-8)".to_string(),
            }
        }
    };

    // Resolve colisões de nome: se já existe um arquivo com o mesmo nome,
    // adiciona um sufixo UUID único antes da extensão.
    let dest_path = resolve_dest_path(&dest_dir, &file_name);

    // Modo simulação: apenas conta como sucesso sem fazer nada
    if args.dry_run {
        return OpResult::Ok { bytes: file.size };
    }

    // Executa a operação
    let result = if args.copy {
        // Cópia: preserva o original
        fs::copy(&file.path, &dest_path).map(|_| ())
    } else {
        // Mover: tenta rename atômico primeiro (instantâneo se mesmo filesystem)
        // Se falhar (cross-device), faz copy + delete
        fs::rename(&file.path, &dest_path).or_else(|_| {
            fs::copy(&file.path, &dest_path)
                .and_then(|_| fs::remove_file(&file.path))
                .map(|_| ())
        })
    };

    match result {
        Ok(_) => OpResult::Ok { bytes: file.size },
        Err(e) => OpResult::Err {
            path: file.path.clone(),
            reason: e.to_string(),
        },
    }
}

/// Resolve o caminho de destino evitando sobrescrever arquivos existentes.
///
/// Se `dir/nome.ext` já existe, tenta `dir/nome_<uuid_curto>.ext`.
fn resolve_dest_path(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);

    if !candidate.exists() {
        return candidate;
    }

    // Separa stem e extensão para inserir o sufixo no lugar certo
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);
    let ext = path.extension().and_then(|e| e.to_str());

    // Gera um sufixo curto (primeiros 8 chars do UUID v4)
    let suffix = &Uuid::new_v4().to_string()[..8];

    let new_name = match ext {
        Some(e) => format!("{}_{}.{}", stem, suffix, e),
        None => format!("{}_{}", stem, suffix),
    };

    dir.join(new_name)
}

// ─── Utilitários ─────────────────────────────────────────────────────────────

/// Formata um número de bytes em representação legível (KB, MB, GB, TB).
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_idx = 0;

    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.2} {}", value, UNITS[unit_idx])
    }
}
