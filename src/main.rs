use core::panic;
use std::{fs::File, io::{Write, Read, self}, process::exit, collections::HashMap, env};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use colored::Colorize;
use dialoguer::{Select, theme::ColorfulTheme, Input};
use figlet_rs::FIGfont;
use config::{Config, File as ConfigFile};

const FILENAME_TOKEN_JSON: &str = ".token";
const FILENAME_CONFIG_JSON: &str = "dataset.json";
const FILENAME_SECRETS_TOML: &str = "secrets.toml";
const FONT: &'static str = include_str!("doom.flf");

#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    token_type: String,
    expires_on: String,
    access_token: String,
}


#[derive(Debug, Serialize, Deserialize)]
struct GuidEntry {
    id: u32,
    #[serde(default)]
    guid: Vec<String>,
}


impl Default for TokenResponse {
    fn default() -> Self {
        TokenResponse {
            token_type: String::new(),
            expires_on: String::new(),
            access_token: String::new(),
        }
    }
}


async fn acquire_new_token(secrets: HashMap<String, String>) -> Result<TokenResponse, String>{

    let url = "https://login.windows.net/common/oauth2/token";
    let params = [
        ("client_id", secrets.get("client_id")),
        ("grant_type", secrets.get("grant_type")),
        ("resource", secrets.get("resource")),
        ("username", secrets.get("username")),
        ("password", secrets.get("password"))
    ];

    let client = reqwest::Client::new();

    let res = client.post(url)
    .body("Something")
    .form(&params)
    .send()
    .await
    .expect("send");

    if res.status().is_success() {
        let token_response: TokenResponse = res.json().await.expect("Falha ao converter JSON.");
        Ok(token_response)
    } else {
        let text_response: String = res.text().await.expect("Falha ao receber mensagem de erro.");
        Err(text_response)

    }

}

async fn send_request_update_dataset(dataset_id: String, token: &TokenResponse) -> Result<reqwest::StatusCode, reqwest::StatusCode> {

    let url = format!("https://api.powerbi.com/v1.0/myorg/datasets/{}/refreshes", dataset_id);
    let access_token = token.access_token.clone();

    let client = reqwest::Client::new();
    let res = client.post(url)
    .bearer_auth(access_token)
    .header("Content-Length", 0)
    .send()
    .await
    .expect("Falha ao enviar solicitação de atualização.");

    if res.status().is_success() {
        Ok(res.status())
    } else {
        Err(res.status())
    }
}

fn validate_token(token: &TokenResponse) -> bool {

    let now: DateTime<Utc> = Utc::now();

    let expire_token: i64 = token.expires_on.trim().parse::<i64>().unwrap_or_default();

    let expire_token_date: DateTime<Utc> = DateTime::from_timestamp(expire_token, 0).unwrap();

    now < expire_token_date
}

fn read_token_file() -> Option<TokenResponse> {

    let current_dir = env::current_dir().expect("Erro ao obter diretório de execução");
    let full_current_dir = current_dir.join(&FILENAME_TOKEN_JSON);
    
    let mut file = match File::open(full_current_dir) {
        Ok(file) => file,
        Err(_) => return None,
    };

    let mut content: String = String::new();
    if let Err(_) = file.read_to_string(&mut content) {
        return None;
    }

    match serde_json::from_str(&content) {
        Ok(token) => token,
        Err(_) => None
    }
}

fn read_config_file() -> Vec<GuidEntry> {

    let mut file = match File::open(FILENAME_CONFIG_JSON) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("{}{}", "Erro ao ler arquivo de configurações\n", e);
            pause();
            exit(1);
        },
    };

    let mut content = String::new();
    file.read_to_string(&mut content).expect("Erro ao ler arquivo de configurações.");

    match serde_json::from_str::<Vec<GuidEntry>>(&content) {
        Ok(entries) => entries,
        Err(_) => {
            eprintln!("Erro ao desserializar arquivo de dataset.");
            pause();
            exit(1);
        }
    }
}

fn read_secrets_file() -> HashMap<String, String>{
    let current_dir = env::current_dir().expect("Erro ao obter diretório de execução");
    let settings_file = current_dir.join(&FILENAME_SECRETS_TOML);

    let settings_builder = Config::builder()
    .add_source(ConfigFile::with_name(&settings_file.to_str().unwrap()))
    .build();

    match settings_builder {
        Ok(settings) => {

            let settings = settings.try_deserialize::<HashMap<String, String>>().unwrap();
            return settings;
        }
        Err(e) => {
            eprintln!("{}{}", "Falha ao ler arquivo de segredos.\n", e);
            pause();
            exit(1);
        }
    }
}

fn export_token(token: &TokenResponse) {
    let filename = FILENAME_TOKEN_JSON;
    let content = serde_json::to_string(&token).unwrap();

    let mut file = match File::create(filename) {
        Ok(file) => file,
        Err(e) => {
            panic!("Falha ao criar arquivo de token.\nErro: {}", e);
        }
    };

    match file.write_all(content.as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            panic!("Erro ao gravar arquivo.\nErro: {}", e);
        }
    }
}

fn pause() {
    let message = "\nPressione ENTER para finalizar\n".yellow();
    println!("{}", message);
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).expect("Falha ao ler entrada do usuário.");
}

fn welcome_message() {
    let standard_font = FIGfont::from_content(FONT).unwrap();
    let figure = standard_font.convert("PowerBI    Updater");
    println!("{}", figure.unwrap());
}

#[tokio::main]
async fn main() {
    
    // Mensagem inicial escrita em Figlet.
    welcome_message();

    // Realiza a leitura do arquivo de senhas e segredos.
    let secrets: HashMap<String, String> = read_secrets_file();

    // Cria uma hashtable para armazenar os valores de maneira mais fácil.
    let mut hash_guid_entries: HashMap<u32, Vec<String>> = HashMap::new();

    /*
    Recupera do arquivo os GUID de atualização.
    Salva cada guid em um novo registro.
    */
    for config in read_config_file() {
        hash_guid_entries.insert(config.id, config.guid);
    }

    let mut token: TokenResponse = TokenResponse::default();
    let mut is_loaded_token: bool = false;

    // Realiza leitura do arquivo com o token salvo (caso houver)
    match read_token_file() {
        // Se o arquivo for lido, será usado o token.
        Some(token_loaded) => {

            // Verifica se o token já perdeu a validade
            if validate_token(&token_loaded) {
                token = token_loaded;
                is_loaded_token = true;
            }
        }
        None => {}
    }

    // Será feito uma tentativa de obtenção de um novo token.
    if is_loaded_token == false {
        match acquire_new_token(secrets).await {
            // Caso o token seja gerado com sucesso.
            Ok(token_loaded) => {
                println!("Novo token gerado !");
                token = token_loaded;
                is_loaded_token = true;
                export_token(&token);
            },
            // Caso ocorra erro ao gerar o novo token.
            Err(_) => {
                eprintln!("Erro ao gerar novo token.\nConsidere validar o arquivo de segredos.");
                pause();
                exit(1);
            }
        }
    }

    // Não foi possível obter o token de nenhuma maneira, o programa encerra.
    if is_loaded_token == false {
        eprintln!("Não foi possível carregar o token de atualização.");
        pause();
        exit(500);
    }
    
    // Opções para seleção do usuário.
    let prompt_options = vec!["Todas empresas", "Uma empresa", "Configurações", "Sair"];
    
    // Exibe o menu iterativo para o usuário.
    let prompt_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Opção: ")
        .default(0)
        .items(&prompt_options)
        .interact()
        .unwrap();

    match prompt_selection {
        0 => {
            // Iterar sobre todos os registros na HashMap.
            for (key, value) in hash_guid_entries.iter() {

                println!("Empresa: {}", key);

                for dataset in value {

                    // Após o token ser carregado, será enviado uma requisição.
                    let update = send_request_update_dataset(dataset.to_string(), &token).await;
        
                    match update {
                        // Caso a requisição retorne sucesso.
                        Ok(_) => {
                            let status = "Aceita".green();
                            println!("\t- Requisição: {}", status);
                        }
                        // Caso a requisição retorne falha.
                        Err(_) => {
                            let status = "Negada".red();
                            eprintln!("\t- Requisição: {}", status);
                        }
                    }
                }
            }
        }
        1 => {
            // Inicia um loop aguardando o usuário digitar uma entrada válida.
            loop {
                // Espera o usuário informar a chave que deseja atualizar.
                let hash_map_key: u32 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("ID Empresa")
                    .interact_text()
                    .expect("Erro ao obter entrada do usuário.");
                
                // Verifica se a chave existe no HashMap.
                match hash_guid_entries.get(&hash_map_key) {
                    Some(value) => {
                        println!("Empresa: {}", hash_map_key);

                        for dataset in value {

                            // Após o token ser carregado, será enviado uma requisição.
                            let update = send_request_update_dataset(dataset.to_string(), &token).await;

                            match update {
                                // Caso a requisição retorne sucesso.
                                Ok(_) => {
                                    let status = "Aceita".green();
                                    println!("\t- Requisição: {}", status);
                                }
                                // Caso a requisição retorne falha.
                                Err(_) => {
                                    let status = "Negada".red();
                                    eprintln!("\t- Requisição: {}", status);
                                }
                            }
                        }
                        break;
                    }
                    None => {
                        // Caso não seja encontrada uma chave, o loop reinicia.
                        eprintln!("Valor não encotrado !");
                    }
                }
            }
        }
        2 => {
            match open::that(FILENAME_CONFIG_JSON) {
                Ok(_) => {}
                Err(_e) => {
                    println!("Falha ao abrir arquivo para edição.");
                }
            }
            println!("{}", "Reinicie a aplicação para aplicar as mudanças.".on_red());
            pause();
            exit(0);
        }
        3 => {
            println!("{}", "Bye".green());
            exit(0);
        }
        _ => {
            eprintln!("Entrada não reconhecida.");
            pause();
            exit(1);
        }
    }

    pause();

}
