## PowerBi Updater

### Descrição

Pequeno programa em CLI para enviar requisições de atualização dos relatórios publicados via PowerBi.

### Instalação

O gerenciador de pacotes padrão do Rust é o `cargo`, para instalar as dependências desse projeto, considere rodar o comando
`cargo build` e para gerar um executável `cargo build --release`

### Configuração

Para iniciar o programa é necessário implementar dois arquivos de configurações juntos ao executável principal (windows - .exe).

#### secrets.toml

O arquivo secrets armazena informações de conexão com o servidores da Microsoft, neste arquivo irá conter senhas e informações sigilosas sobre o login da sua conta com PowerBI Pro.

```toml
client_id = ""
grant_type = "password"
resource = "https://analysis.windows.net/powerbi/api"
username = "" 
password = ""
```

* dataset.json
...
