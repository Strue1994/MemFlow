# OPT-08: Add CLI Auto-completion

## 目标

为 CLI 添加 shell 自动补全支持。

## 当前状态

没有自动补全。

## 实现方案

1. **生成 completion scripts**
   ```rust
   use clap::{App, Arg};
   
   fn generate_completions(app: &App) {
       for shell in &[Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell] {
           let mut buf = Vec::new();
           app.write_long_help(&mut buf).unwrap();
           // 生成 completion
       }
   }
   ```

2. **添加 completion 命令**
   ```rust
   #[derive(Clap)]
   pub enum Commands {
       #[clap(name = "completion", about = "Generate shell completions")]
       Completion {
           shell: Shell,
           #[clap(long, short)]
           output: Option<String>,
       },
   }
   
   fn handle_completion(shell: Shell, output: Option<String>) -> Result<()> {
       let mut app = build_cli();
       let mut buf = Vec::new();
       app.write_long_help(&mut buf)?;
       
       let completion = match shell {
           Shell::Bash => generate_bash(&app),
           Shell::Zsh => generate_zsh(&app),
           Shell::Fish => generate_fish(&app),
           Shell::PowerShell => generate_powershell(&app),
       };
       
       match output {
           Some(path) => std::fs::write(&path, completion)?,
           None => println!("{}", completion),
       }
       Ok(())
   }
   ```

3. **安装脚本**
   ```bash
   # Bash
   memflow completion bash > /etc/bash_completion.d/memflow
   
   # Zsh
   memflow completion zsh > ~/.zsh/completions/_memflow
   
   # Fish
   memflow completion fish > ~/.config/fish/completions/memflow.fish
   ```

## 影响文件

- CLI 主程序

## 验证方法

运行 `memflow completion bash` 测试。

## 优先级

LOW - 用户体验