//! O MCP **ao vivo**: o aplicativo hospeda o motor de ferramentas num socket local (só nesta máquina, pasta `0700`, nada de
//! rede) e o `feathercad-mcp` que o Claude Code abre vira um proxy fino: com o aplicativo aberto, cada `tools/call` vai para
//! lá e age sobre o desenho que está na tela; sem ele, o servidor trabalha sozinho, como sempre. Uma mensagem JSON por linha,
//! as mesmas do stdio. Só Unix por enquanto (no Windows o servidor segue independente).

use std::path::PathBuf;

/// Onde o aplicativo escuta: `$XDG_RUNTIME_DIR/feathercad/mcp.sock` (ou uma pasta própria em `/tmp`).
pub fn caminho_do_socket() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| {
        let usuario = std::env::var("USER").unwrap_or_else(|_| "usuario".into());
        std::env::temp_dir().join(format!("feathercad-{usuario}"))
    });
    base.join("feathercad").join("mcp.sock")
}

#[cfg(unix)]
mod unix {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};

    use serde_json::Value;

    use super::caminho_do_socket;

    /// O lado do `feathercad-mcp`: conecta por demanda e repassa uma linha de cada vez.
    #[derive(Default)]
    pub struct Cliente {
        conexao: Option<(UnixStream, BufReader<UnixStream>)>,
    }

    impl Cliente {
        fn conectar(&mut self) -> bool {
            if self.conexao.is_some() {
                return true;
            }
            if std::env::var("FEATHERCAD_VIVO").is_ok_and(|v| v == "0") {
                return false;
            }
            let Ok(s) = UnixStream::connect(caminho_do_socket()) else { return false };
            let Ok(leitura) = s.try_clone() else { return false };
            self.conexao = Some((s, BufReader::new(leitura)));
            true
        }

        /// `None` = o aplicativo não está aberto (quem chamou atende sozinho). `Some(Err)` = a conversa caiu no meio
        /// (a ferramenta pode ou não ter rodado: não se repete sozinho). `Some(Ok(linha))` = a resposta do aplicativo.
        pub fn repassar(&mut self, linha: &str) -> Option<Result<String, String>> {
            if !self.conectar() {
                return None;
            }
            let (escrita, leitura) = self.conexao.as_mut()?;
            let r = writeln!(escrita, "{linha}").and_then(|()| escrita.flush()).and_then(|()| {
                let mut resposta = String::new();
                match leitura.read_line(&mut resposta)? {
                    0 => Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "o aplicativo fechou a conexão")),
                    _ => Ok(resposta),
                }
            });
            match r {
                Ok(resposta) => Some(Ok(resposta.trim_end().to_string())),
                Err(e) => {
                    self.conexao = None;
                    Some(Err(format!("a conversa com o aplicativo FeatherCAD caiu ({e}); confira o desenho na tela antes de repetir")))
                }
            }
        }
    }

    /// O lado do aplicativo: escuta no socket e chama `tratar` para cada mensagem (uma thread por conexão).
    /// Se já há outro aplicativo escutando, não faz nada (devolve `Ok(false)`).
    pub fn servir(tratar: impl Fn(Value) -> Option<Value> + Send + Sync + 'static) -> std::io::Result<bool> {
        let caminho = caminho_do_socket();
        if UnixStream::connect(&caminho).is_ok() {
            return Ok(false);
        }
        if let Some(pasta) = caminho.parent() {
            std::fs::create_dir_all(pasta)?;
            std::fs::set_permissions(pasta, std::fs::Permissions::from_mode(0o700))?;
        }
        let _ = std::fs::remove_file(&caminho);
        let ouvinte = UnixListener::bind(&caminho)?;
        std::fs::set_permissions(&caminho, std::fs::Permissions::from_mode(0o600))?;
        let tratar = std::sync::Arc::new(tratar);
        std::thread::spawn(move || {
            for conexao in ouvinte.incoming().flatten() {
                let tratar = tratar.clone();
                std::thread::spawn(move || {
                    let Ok(leitura) = conexao.try_clone() else { return };
                    let mut escrita = conexao;
                    for linha in BufReader::new(leitura).lines() {
                        let Ok(linha) = linha else { break };
                        let Ok(msg) = serde_json::from_str::<Value>(&linha) else { continue };
                        if let Some(r) = tratar(msg) {
                            if writeln!(escrita, "{r}").and_then(|()| escrita.flush()).is_err() {
                                break;
                            }
                        }
                    }
                });
            }
        });
        Ok(true)
    }

    /// Apaga o arquivo do socket (ao fechar o aplicativo).
    pub fn encerrar() {
        let _ = std::fs::remove_file(caminho_do_socket());
    }
}

#[cfg(unix)]
pub use unix::{encerrar, servir, Cliente};

#[cfg(not(unix))]
mod outros {
    use serde_json::Value;

    #[derive(Default)]
    pub struct Cliente;

    impl Cliente {
        pub fn repassar(&mut self, _linha: &str) -> Option<Result<String, String>> {
            None
        }
    }

    pub fn servir(_tratar: impl Fn(Value) -> Option<Value> + Send + Sync + 'static) -> std::io::Result<bool> {
        Ok(false)
    }

    pub fn encerrar() {}
}

#[cfg(not(unix))]
pub use outros::{encerrar, servir, Cliente};

#[cfg(all(test, unix))]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn cliente_e_servidor_conversam_por_linha() {
        let pasta = std::env::temp_dir().join(format!("fcad_vivo_{}", std::process::id()));
        std::env::set_var("XDG_RUNTIME_DIR", &pasta);
        assert!(servir(|m| Some(json!({ "eco": m["id"] }))).unwrap());
        assert!(!servir(|_| None).unwrap(), "um segundo aplicativo não toma o lugar do primeiro");
        let mut c = Cliente::default();
        let r = c.repassar(r#"{"id":7}"#).unwrap().unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&r).unwrap()["eco"], 7);
        encerrar();
        let mut outro = Cliente::default();
        assert!(outro.repassar("{}").is_none(), "sem aplicativo escutando, quem chamou atende sozinho");
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
