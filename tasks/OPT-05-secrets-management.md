# OPT-05: Implement Secrets Management

## 目标

实现安全的 secrets 管理，不要在代码或配置文件中明文存储 secrets。

## 当前状态

可能使用环境变量或配置文件存储 secrets。

## 实现方案

1. **使用 HashiCorp Vault 集成**
   ```rust
   use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
   use vaultrs::kv2;
   
   pub struct SecretsManager {
       client: VaultClient,
   }
   
   impl SecretsManager {
       pub fn new(addr: &str, token: &str) -> Result<Self> {
           let settings = VaultClientSettingsBuilder::new()
               .address(addr)
               .token(token)
               .build();
           let client = VaultClient::new(settings)?;
           Ok(Self { client })
       }
       
       pub fn get_secret(&self, path: &str, key: &str) -> Result<String> {
           let secret = kv2::read(&self.client, path)?;
           Ok(secret.data.get(key).cloned().unwrap())
       }
   }
   ```

2. **使用 AWS Secrets Manager**
   ```rust
   use aws_sdk_secretsmanager::{Client, Config, Region};
   
   pub struct AwsSecrets {
       client: Client,
   }
   
   impl AwsSecrets {
       pub fn new(region: Region) -> Self {
           let config = Config::builder().region(region).build();
           Self { client: Client::new(config) }
       }
       
       pub fn get(&self, name: &str) -> Result<String> {
           let output = self.client.get_secret_value()
               .secret_id(name)
               .send()
               .await?;
           Ok(output.secret_string().unwrap())
       }
   }
   ```

3. **使用本地加密存储 (开发用)**
   ```rust
   use ring::aead::{Aad, LessSafeKey, Nonce};
   use ring::rand::{SecureRandom, SystemRandom};
   
   pub struct LocalSecrets {
       key: [u8; 32],
   }
   
   impl LocalSecrets {
       pub fn new(master_password: &str) -> Self {
           let mut key = [0u8; 32];
           key.copy_from_slice(&sha256(master_password.as_bytes())[..32]);
           Self { key }
       }
       
       pub fn encrypt(&self, plaintext: &str) -> Vec<u8> {
           // 使用 AES-GCM 加密
       }
   }
   ```

## 影响文件

- 新建 `executor/src/secrets.rs`
- `executor/src/lib.rs`

## 验证方法

无明文 secrets 在代码库中。

## 优先级

HIGH - 安全需求