//! Domínio viário/ferroviário: alinhamento horizontal (retas, clotoides e curvas circulares), superelevação,
//! greide e a seção da via. Tudo é conta em Rust; as ferramentas MCP só leem JSON, chamam isto e desenham.

pub mod ajuste;
pub mod alinhamento;
pub mod greide;
pub mod superelevacao;
