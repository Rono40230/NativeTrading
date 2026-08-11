//! Gestion de la session IG Markets REST API.
//! Les credentials sont lus depuis SQLite (jamais depuis le code ou .env).
//! La session (CST + X-SECURITY-TOKEN) est renouvelée automatiquement après 6h.

use anyhow::{anyhow, Result};
use db::Database;
use std::sync::Arc;
use std::time::{Duration, Instant};

const SESSION_TTL: Duration = Duration::from_secs(5 * 3600); // 5h par sécurité (IG = 6h)

// ─── Types désérialisation ────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct IgSessionResponse {
    #[serde(rename = "lightstreamerEndpoint")]
    lightstreamer_endpoint: Option<String>,
    #[serde(rename = "currentAccountId")]
    current_account_id: Option<String>,
}

// ─── Struct principale ────────────────────────────────────────────────────────

pub struct IgSession {
    client: reqwest::Client,
    /// Token de session court (CST)
    cst: Option<String>,
    /// Token de sécurité (X-SECURITY-TOKEN)
    token: Option<String>,
    /// Instant du dernier login réussi
    derniere_connexion: Option<Instant>,
    /// Environnement : "demo" ou "live"
    env: String,
    /// Endpoint Lightstreamer fourni par IG au login (ex: https://apd.marketdatasystems.com)
    pub lightstreamer_endpoint: Option<String>,
    /// ID du compte courant (ex: "ILZC1") — utilisé comme LS_user
    pub account_id: Option<String>,
}

impl IgSession {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            cst: None,
            token: None,
            derniere_connexion: None,
            env: "demo".into(),
            lightstreamer_endpoint: None,
            account_id: None,
        }
    }

    fn base_url(&self) -> &'static str {
        // IG utilise la même URL pour demo et live (distinction par les credentials).
        // Les comptes demo créés via l'interface web utilisent api.ig.com
        // uniquement les comptes API dédiés utilisent demo-api.ig.com
        "https://api.ig.com/gateway/deal"
    }

    /// Vérifie si la session est encore valide (moins de 5h).
    fn est_valide(&self) -> bool {
        match self.derniere_connexion {
            Some(t) => t.elapsed() < SESSION_TTL && self.cst.is_some() && self.token.is_some(),
            None => false,
        }
    }

    /// Tente un login avec les credentials stockés en DB.
    /// Retourne Err si les credentials sont absents ou si IG refuse.
    pub async fn login(&mut self, db: &Database) -> Result<()> {
        let api_key = db
            .lire_config("ig_api_key")
            .await?
            .ok_or_else(|| anyhow!("ig_api_key non configurée dans les Settings"))?;
        let username = db
            .lire_config("ig_username")
            .await?
            .ok_or_else(|| anyhow!("ig_username non configuré dans les Settings"))?;
        let password = db
            .lire_config("ig_password")
            .await?
            .ok_or_else(|| anyhow!("ig_password non configuré dans les Settings"))?;

        let env = db
            .lire_config("ig_env")
            .await?
            .unwrap_or_else(|| "demo".into());
        self.env = env;

        let url = format!("{}/session", self.base_url());

        let body = serde_json::json!({
            "identifier": username,
            "password": password,
        });

        let resp = self
            .client
            .post(&url)
            .header("X-IG-API-KEY", &api_key)
            .header("Content-Type", "application/json; charset=UTF-8")
            .header("Accept", "application/json; charset=UTF-8")
            .header("Version", "2")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let texte = resp.text().await.unwrap_or_default();
            return Err(anyhow!("IG login échoué ({}): {}", status, texte));
        }

        let cst = resp
            .headers()
            .get("CST")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .ok_or_else(|| anyhow!("IG login: header CST absent"))?;

        let token = resp
            .headers()
            .get("X-SECURITY-TOKEN")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .ok_or_else(|| anyhow!("IG login: header X-SECURITY-TOKEN absent"))?;

        // Consommer le body pour récupérer lightstreamerEndpoint et accountId
        if let Ok(body) = resp.json::<IgSessionResponse>().await {
            self.lightstreamer_endpoint = body.lightstreamer_endpoint;
            self.account_id = body.current_account_id;
        }

        self.cst = Some(cst);
        self.token = Some(token);
        self.derniere_connexion = Some(Instant::now());

        tracing::info!("IG Markets session établie (env={})", self.env);
        Ok(())
    }

    /// Retourne les headers HTTP nécessaires pour chaque requête IG.
    /// Relogin automatique si la session a expiré.
    /// Retourne Err si les credentials manquent ou si IG est inaccessible.
    pub async fn headers(&mut self, db: &Arc<Database>) -> Result<reqwest::header::HeaderMap> {
        if !self.est_valide() {
            self.login(db).await?;
        }

        let cst = self
            .cst
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("CST absent après login"))?;
        let token = self
            .token
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("X-SECURITY-TOKEN absent après login"))?;

        let api_key = db.lire_config("ig_api_key").await?.unwrap_or_default();

        let mut map = reqwest::header::HeaderMap::new();
        map.insert("X-IG-API-KEY", api_key.parse()?);
        map.insert("CST", cst.parse()?);
        map.insert("X-SECURITY-TOKEN", token.parse()?);
        map.insert("Accept", "application/json; charset=UTF-8".parse()?);
        map.insert("Content-Type", "application/json; charset=UTF-8".parse()?);

        Ok(map)
    }

    /// Retourne l'URL de base REST IG.
    pub fn url(&self) -> &'static str {
        self.base_url()
    }

    /// Retourne le client HTTP partagé.
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Retourne (cst, token) si la session est active — pour vérification statut.
    #[allow(dead_code)]
    pub fn est_connecte(&self) -> bool {
        self.est_valide()
    }

    /// Retourne le CST token actuel (pour Lightstreamer LS_password).
    pub fn cst(&self) -> Option<&str> {
        self.cst.as_deref()
    }

    /// Retourne le X-SECURITY-TOKEN actuel (nécessaire pour Lightstreamer LS_password avec IG live).
    pub fn security_token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Force un relogin (utilisé par le endpoint /api/ig/status — bouton Tester).
    /// Si le login échoue, la session précédente valide est restaurée.
    pub async fn tester_connexion(&mut self, db: &Arc<Database>) -> Result<()> {
        // Sauvegarder l'état actuel avant de reset
        let saved_cst = self.cst.clone();
        let saved_token = self.token.clone();
        let saved_connexion = self.derniere_connexion;
        // Reset pour forcer un vrai login réseau
        self.cst = None;
        self.token = None;
        self.derniere_connexion = None;
        match self.login(db).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Restaurer la session précédente si elle était valide
                if saved_cst.is_some() && saved_token.is_some() {
                    self.cst = saved_cst;
                    self.token = saved_token;
                    self.derniere_connexion = saved_connexion;
                    tracing::warn!(
                        "IG test connexion échoué — session précédente restaurée: {}",
                        e
                    );
                }
                Err(e)
            }
        }
    }

    /// Invalide la session (utilisé quand les credentials sont mis à jour en Settings).
    /// Le prochain appel à headers() déclenchera un nouveau login.
    pub fn reset(&mut self) {
        self.cst = None;
        self.token = None;
        self.derniere_connexion = None;
        self.lightstreamer_endpoint = None;
        self.account_id = None;
        tracing::info!("IG Markets: session invalidée (reload au prochain appel)");
    }
}
