//! Ícone e dados do executável. A versão vem de versao.json.
fn main() {
    println!("cargo:rerun-if-changed=versao.json");
    println!("cargo:rerun-if-changed=recursos/icone.ico");
    let texto = std::fs::read_to_string("versao.json").expect("versao.json não encontrado");
    let dados: serde_json::Value = serde_json::from_str(&texto).expect("versao.json inválido");
    let versao = dados["versao"].as_str().expect("versao.json sem versao");
    let (maior, menor) = versao.split_once('.').expect("versao fora do formato 0.00");
    let numero = maior.parse::<u64>().unwrap() << 48 | menor.parse::<u64>().unwrap() << 32;

    let mut r = winresource::WindowsResource::new();
    r.set_icon("recursos/icone.ico")
        .set("ProductName", "MCP PremoSienge")
        .set("FileDescription", "MCP PremoSienge")
        .set("ProductVersion", versao)
        .set("FileVersion", versao)
        .set_version_info(winresource::VersionInfo::PRODUCTVERSION, numero)
        .set_version_info(winresource::VersionInfo::FILEVERSION, numero)
        .set_language(0x0416); // português do Brasil
    r.compile().expect("não foi possível gerar o ícone do executável");
}
