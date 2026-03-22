# 📁 file-organizer

**Organizador de arquivos por extensão — rápido, paralelo e seguro**

---

## Por que Rust?

Esta ferramenta foi escrita em **Rust** por razões técnicas objetivas para este caso de uso específico:

| Critério | Rust | Python | Go | C |
|---|---|---|---|---|
| Velocidade de execução | ⚡ Nativa (igual a C) | 🐢 Lento (interpretado) | ⚡ Muito rápido | ⚡ Nativa |
| Paralelismo seguro | ✅ Rayon (sem data races) | ⚠️ GIL limita threads | ✅ Goroutines | ❌ Manual/perigoso |
| Binário único | ✅ Sim | ❌ Precisa do Python | ✅ Sim | ✅ Sim |
| Segurança de memória | ✅ Garantida pelo compilador | ✅ GC | ✅ GC | ❌ Manual |
| Sem runtime | ✅ Sim | ❌ Precisa do runtime | ❌ Pequeno runtime | ✅ Sim |
| Facilidade de uso | ✅ Cargo (build system) | ✅ pip | ✅ go mod | ❌ Makefiles/cmake |

**Resumo**: Rust entrega a velocidade de C com a segurança de linguagens modernas, e o Cargo (gerenciador de pacotes) torna o build trivial. Para uma ferramenta que vai mover milhares de arquivos em paralelo, é a escolha ideal.

---

## Dependências (crates)

### `rayon` v1.10
**O que é**: Biblioteca de paralelismo de dados para Rust. Permite iterar sobre coleções usando múltiplos núcleos da CPU com uma mudança mínima no código (`iter()` → `par_iter()`).

**Por que foi escolhida**: Mover/copiar milhares de arquivos é uma operação I/O-bound que se beneficia imensamente de paralelismo. O Rayon distribui automaticamente o trabalho entre todos os núcleos disponíveis sem que você precise gerenciar threads manualmente.

**Como funciona internamente**: Usa um algoritmo de *work-stealing* — threads ociosas "roubam" trabalho de threads ocupadas, garantindo balanceamento de carga ideal.

**Site**: https://docs.rs/rayon

---

### `clap` v4.5 (feature: derive)
**O que é**: Parser de argumentos de linha de comando para Rust. É o padrão de facto do ecossistema Rust para CLIs.

**Por que foi escolhida**: Gera automaticamente a mensagem de `--help`, valida tipos de argumentos, e permite definir a interface CLI via anotações (derive macros) sem código boilerplate.

**Feature `derive`**: Permite usar `#[derive(Parser)]` e `#[arg(...)]` para declarar argumentos como campos de uma struct, em vez de código imperativo.

**Site**: https://docs.rs/clap

---

### `indicatif` v0.17
**O que é**: Biblioteca para barras de progresso e spinners no terminal.

**Por que foi escolhida**: Para operações longas com milhares de arquivos, feedback visual em tempo real é essencial para o usuário saber que o programa está funcionando e estimar o tempo restante.

**Site**: https://docs.rs/indicatif

---

### `colored` v2.1
**O que é**: Adiciona cores e estilos ANSI ao output do terminal de forma ergonômica.

**Por que foi escolhida**: Diferencia visualmente sucesso (verde), avisos (amarelo) e erros (vermelho) sem depender de sequências ANSI manuais.

**Site**: https://docs.rs/colored

---

### `uuid` v1.8 (feature: v4)
**O que é**: Geração de UUIDs (Universally Unique Identifiers).

**Por que foi escolhida**: Quando dois arquivos com o mesmo nome (mas de pastas diferentes) precisam ir para o mesmo destino, um sufixo UUID garante que nunca haverá sobrescrita acidental.

**Feature `v4`**: UUIDs versão 4 são gerados com entropia aleatória — probabilidade de colisão astronomicamente baixa.

**Site**: https://docs.rs/uuid

---

## Pré-requisitos

### Instalar Rust no Manjaro

Rust é instalado via `rustup` — o instalador oficial que também gerencia versões e toolchains.

```bash
# 1. Instala o rustup (não use o pacote do repositório do Manjaro — é desatualizado)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Durante a instalação, escolha a opção **1) Proceed with standard installation**.

```bash
# 2. Recarrega as variáveis de ambiente no shell atual
source "$HOME/.cargo/env"

# 3. Verifica a instalação
rustc --version   # ex: rustc 1.77.0 (aedd173a2 2024-03-17)
cargo --version   # ex: cargo 1.77.0 (814aced6a 2024-03-06)
```

O `rustup` instala:
- **`rustc`**: o compilador Rust
- **`cargo`**: o build system e gerenciador de pacotes
- **`rustup`**: para gerenciar versões do toolchain

---

## Instalação do file-organizer

### Opção A: Compilar do código-fonte (recomendado)

```bash
# 1. Clone ou baixe o projeto
#    Se você tiver o código em um diretório:
cd /caminho/para/file-organizer

# 2. Compile em modo release (otimizações máximas)
cargo build --release

# O binário estará em:
ls -lh target/release/file-organizer
```

A compilação leva entre 30 segundos e 2 minutos na primeira vez (baixa e compila as dependências). Compilações subsequentes são incrementais e muito mais rápidas.

### Opção B: Instalar globalmente no sistema

```bash
# Compila e instala em ~/.cargo/bin/ (que já deve estar no seu PATH)
cargo install --path .

# Agora você pode usar de qualquer lugar:
file-organizer --help
```

### Verificar instalação

```bash
file-organizer --version
# file-organizer 1.0.0
```

---

## Uso

### Sintaxe básica

```
file-organizer --source <ORIGEM> --dest <DESTINO> [OPÇÕES]
```

### Argumentos obrigatórios

| Argumento | Atalho | Descrição |
|---|---|---|
| `--source <DIR>` | `-s` | Diretório a ser varrido recursivamente |
| `--dest <DIR>` | `-d` | Diretório raiz onde as pastas por extensão serão criadas |

### Opções

| Argumento | Atalho | Padrão | Descrição |
|---|---|---|---|
| `--dry-run` | — | false | Simula tudo sem mover/copiar nada |
| `--copy` | `-c` | false | Copia em vez de mover (mantém originais) |
| `--threads <N>` | `-t` | todos os núcleos | Número de threads paralelas |
| `--quiet` | — | false | Suprime output visual (útil em scripts) |
| `--include-hidden` | — | false | Inclui arquivos/pastas que começam com `.` |
| `--help` | `-h` | — | Exibe ajuda |
| `--version` | `-V` | — | Exibe versão |

---

## Exemplos de uso

### Caso de uso típico: organizar um HD externo

Situação: seu HD está montado em `/mnt/hd` e você quer organizar a pasta `/mnt/hd/Downloads`.

```bash
file-organizer --source /mnt/hd/Downloads --dest /mnt/hd
```

**Resultado**: Os arquivos serão movidos para:
```
/mnt/hd/
├── jpg/
│   ├── foto1.jpg
│   └── foto2.jpg
├── mp4/
│   └── video.mp4
├── pdf/
│   └── documento.pdf
├── zip/
│   └── backup.zip
└── sem-extensao/
    └── makefile
```

---

### Simular antes de executar (dry-run)

```bash
# Veja o que seria feito sem mover nada
file-organizer --source /mnt/hd/Downloads --dest /mnt/hd --dry-run
```

---

### Copiar em vez de mover

```bash
# Mantém os arquivos originais e cria cópias organizadas
file-organizer --source /mnt/hd/Downloads --dest /mnt/hd/organizado --copy
```

---

### Organizar múltiplas pastas de uma vez

```bash
# Usando um loop bash
for pasta in /mnt/hd/Fotos /mnt/hd/Videos /mnt/hd/Documentos; do
    file-organizer --source "$pasta" --dest /mnt/hd
done
```

---

### Controlar número de threads

```bash
# Usar apenas 4 threads (útil para HDs mecânicos, onde paralelismo excessivo prejudica)
file-organizer --source /mnt/hd/Downloads --dest /mnt/hd --threads 4

# Para SSDs, use todos os núcleos (comportamento padrão)
file-organizer --source /mnt/hd/Downloads --dest /mnt/hd
```

> **Dica importante para HDs mecânicos (HDD)**: HDDs têm cabeça de leitura física. Muitas threads lendo partes aleatórias do disco simultaneamente pode causar *thrashing* e deixar a operação mais lenta. Para HDDs, teste com `--threads 2` ou `--threads 4`. Para SSDs ou NVMe, use o padrão (todos os núcleos).

---

### Uso em script (sem output visual)

```bash
# Silencioso, ideal para cron jobs ou automação
file-organizer --source /mnt/hd/Downloads --dest /mnt/hd --quiet

# Verifica o código de saída
if [ $? -eq 0 ]; then
    echo "Organização concluída com sucesso"
fi
```

---

### Incluir arquivos ocultos

```bash
# Por padrão, arquivos como .bashrc, .gitignore são ignorados
# Para incluí-los:
file-organizer --source /mnt/hd --dest /mnt/hd/organizado --include-hidden
```

---

## Comportamento detalhado

### Como a extensão é determinada

| Arquivo | Extensão detectada | Pasta de destino |
|---|---|---|
| `foto.JPG` | `jpg` | `/dest/jpg/` |
| `Foto.JPEG` | `jpeg` | `/dest/jpeg/` |
| `video.MP4` | `mp4` | `/dest/mp4/` |
| `makefile` | *(nenhuma)* | `/dest/sem-extensao/` |
| `.hidden` | *(nenhuma)* | `/dest/sem-extensao/` *(se incluído)* |
| `arquivo.tar.gz` | `gz` | `/dest/gz/` |

> **Nota**: A extensão sempre é normalizada para **minúsculas**. `foto.JPG` e `imagem.jpg` vão para a mesma pasta `/dest/jpg/`.

---

### Como colisões de nome são tratadas

Se dois arquivos de pastas diferentes têm o mesmo nome:

```
/mnt/hd/Fotos/2023/foto.jpg  →  /mnt/hd/jpg/foto.jpg
/mnt/hd/Fotos/2024/foto.jpg  →  /mnt/hd/jpg/foto_a3f9b2c1.jpg  ← sufixo UUID curto
```

Arquivos existentes **nunca são sobrescritos**.

---

### Como o mover funciona (internamente)

1. **Tenta `rename()`**: operação atômica e instantânea — funciona quando origem e destino estão no **mesmo filesystem**
2. **Se falhar, faz `copy()` + `delete()`**: usado quando origem e destino estão em filesystems diferentes (ex: mover de `/mnt/hd1` para `/mnt/hd2`)

---

## Performance esperada

Benchmarks aproximados em hardware típico:

| Hardware | Arquivos | Tempo estimado |
|---|---|---|
| SSD NVMe, i7 8-core | 100.000 arquivos, 50GB | ~45 segundos |
| SSD SATA, i5 4-core | 50.000 arquivos, 20GB | ~60 segundos |
| HDD 7200rpm, i5 4-core, `--threads 4` | 30.000 arquivos, 100GB | ~5-15 minutos |

> O gargalo é sempre o disco, não a CPU. Rust e Rayon garantem que a CPU nunca seja o limitante.

---

## Solução de problemas

### "command not found: file-organizer"

```bash
# Verifica se ~/.cargo/bin está no PATH
echo $PATH | grep cargo

# Se não estiver, adicione ao ~/.bashrc ou ~/.zshrc:
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### "Permission denied" em alguns arquivos

O programa ignora silenciosamente arquivos sem permissão de leitura e continua. Os erros são listados no relatório final. Para ver todos os arquivos problemáticos, verifique as permissões:

```bash
# Lista arquivos sem permissão de leitura no diretório
find /mnt/hd -type f ! -readable 2>/dev/null
```

### Compilação falha com "linker not found"

```bash
# Instala ferramentas de compilação C (necessárias para alguns crates)
sudo pacman -S base-devel
```

### Compilação lenta

A primeira compilação é lenta porque baixa e compila todas as dependências. Compilações subsequentes são incrementais. Em um CI/CD, use cache do diretório `target/`.

---

## Estrutura do projeto

```
file-organizer/
├── Cargo.toml        # Manifesto do projeto: dependências, metadados, perfis de build
├── Cargo.lock        # Versões exatas das dependências (commitar em projetos binários)
├── src/
│   └── main.rs       # Código-fonte completo
└── target/
    └── release/
        └── file-organizer    # Binário compilado (gerado após `cargo build --release`)
```

---

## Como atualizar

```bash
# Atualiza o Rust para a versão mais recente
rustup update stable

# Recompila com a nova versão
cargo build --release
```

---

## Desinstalar

```bash
# Remove o binário instalado globalmente
cargo uninstall file-organizer

# Remove o Rust completamente do sistema
rustup self uninstall
```
