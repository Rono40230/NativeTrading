//! §Capture PNG des alertes SMC (chantier MOB owner 17/10) — rendu SERVEUR
//! du graphique concerné : ~120 bougies, bande de zone en surbrillance,
//! ligne du prix. Pas de capture d'écran desktop (fragile, dépend de la
//! page ouverte) : ce rendu est déterministe, disponible quelle que soit
//! la face affichée, et sert à la fois l'APK et tout futur canal.
//!
//! Sans texte volontairement (l'app affiche asset/TF au-dessus de l'image
//! — ses données sont plus riches qu'un raster) : zéro dépendance police.

use image::{Rgb, RgbImage};

/// Couleurs (fond sombre app) : bougies Modern vert/rouge, zone ambre
/// (achat) ou bleu (vente), prix blanc.
const FOND: [u8; 3] = [10, 14, 39];
const HAUSSIER: [u8; 3] = [16, 185, 129];
const BAISSIER: [u8; 3] = [239, 68, 68];
const ZONE_ACHAT: [u8; 3] = [245, 158, 11];
const ZONE_VENTE: [u8; 3] = [59, 130, 246];
const PRIX: [u8; 3] = [255, 255, 255];
const GRILLE: [u8; 3] = [30, 38, 70];

/// Une bougie (ts, open, high, low, close) — fermée.
pub type BougiePng = (i64, f64, f64, f64, f64);

#[derive(Clone)]
pub struct ParamsCapture {
    pub bougies: Vec<BougiePng>,
    pub zone_haut: f64,
    pub zone_bas: f64,
    pub achat: bool,
    /// Prix au moment de l'alerte (ligne blanche).
    pub prix: f64,
}

const L: u32 = 800;
const H: u32 = 450;
const MARGE: u32 = 8;

fn melange(base: [u8; 3], couleur: [u8; 3], poids: f64) -> Rgb<u8> {
    Rgb([
        (base[0] as f64 * (1.0 - poids) + couleur[0] as f64 * poids) as u8,
        (base[1] as f64 * (1.0 - poids) + couleur[1] as f64 * poids) as u8,
        (base[2] as f64 * (1.0 - poids) + couleur[2] as f64 * poids) as u8,
    ])
}

fn px(img: &mut RgbImage, x: i64, y: i64, c: [u8; 3]) {
    if x >= 0 && y >= 0 && (x as u32) < L && (y as u32) < H {
        img.put_pixel(x as u32, y as u32, Rgb(c));
    }
}

fn rect_plein(img: &mut RgbImage, x0: i64, y0: i64, x1: i64, y1: i64, c: [u8; 3]) {
    for y in y0.min(y1)..=y0.max(y1) {
        for x in x0.min(x1)..=x0.max(x1) {
            px(img, x, y, c);
        }
    }
}

fn rect_translucide(img: &mut RgbImage, x0: i64, y0: i64, x1: i64, y1: i64, c: [u8; 3]) {
    for y in y0.min(y1)..=y0.max(y1) {
        for x in x0.min(x1)..=x0.max(x1) {
            if let Some(p) = img.get_pixel_checked(x as u32, y as u32) {
                let base = [p.0[0], p.0[1], p.0[2]];
                img.put_pixel(x as u32, y as u32, melange(base, c, 0.22));
            }
        }
    }
}

fn hline_pointille(img: &mut RgbImage, y: i64, c: [u8; 3]) {
    let mut x = 0i64;
    while x < L as i64 {
        for k in 0..4 {
            px(img, x + k, y, c);
        }
        x += 8;
    }
}

/// Rend la capture en PNG (octets). `None` si pas assez de bougies.
pub fn rendre(p: &ParamsCapture) -> Option<Vec<u8>> {
    if p.bougies.len() < 5 {
        return None;
    }
    // Échelle verticale : extrêmes des bougies + zone + prix, 8 % de marge.
    let mut bas = p
        .bougies
        .iter()
        .map(|b| b.3)
        .chain([p.zone_bas, p.zone_haut, p.prix])
        .fold(f64::INFINITY, f64::min);
    let mut haut = p
        .bougies
        .iter()
        .map(|b| b.2)
        .chain([p.zone_bas, p.zone_haut, p.prix])
        .fold(f64::NEG_INFINITY, f64::max);
    let marge_p = (haut - bas).max(1e-9) * 0.08;
    bas -= marge_p;
    haut += marge_p;
    let y_prix = |v: f64| -> i64 { (H as f64 - MARGE as f64 - (v - bas) / (haut - bas) * (H - 2 * MARGE) as f64) as i64 };

    let mut img = RgbImage::from_pixel(L, H, Rgb(FOND));
    // Grille horizontale discrète (5 lignes).
    for i in 1..5 {
        let y = (H as f64 * i as f64 / 5.0) as i64;
        for x in 0..L as i64 {
            px(&mut img, x, y, GRILLE);
        }
    }

    // Bougies : mèche 1 px, corps proportionnel.
    let n = p.bougies.len() as f64;
    let pas = (L - 2 * MARGE) as f64 / n;
    let largeur = ((pas * 0.62).floor() as i64).max(1);
    for (i, (_, o, h, l, c)) in p.bougies.iter().enumerate() {
        let x0 = MARGE as i64 + (i as f64 * pas) as i64 + ((pas - largeur as f64) / 2.0) as i64;
        let couleur = if c >= o { HAUSSIER } else { BAISSIER };
        let xm = x0 + largeur / 2;
        for y in y_prix(*h)..=y_prix(*l) {
            px(&mut img, xm, y, couleur); // mèche
        }
        rect_plein(&mut img, x0, y_prix(*o), x0 + largeur, y_prix(*c), couleur);
    }

    // Bande de zone (sur toute la largeur) + bordures.
    let zc = if p.achat { ZONE_ACHAT } else { ZONE_VENTE };
    let (zy0, zy1) = (y_prix(p.zone_haut), y_prix(p.zone_bas));
    rect_translucide(&mut img, 0, zy0, L as i64 - 1, zy1, zc);
    for x in 0..L as i64 {
        px(&mut img, x, zy0, zc);
        px(&mut img, x, zy1, zc);
    }

    // Prix de l'alerte : blanc pointillé.
    hline_pointille(&mut img, y_prix(p.prix), PRIX);

    let mut octets = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut octets), image::ImageFormat::Png)
        .ok()?;
    Some(octets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bougies() -> Vec<BougiePng> {
        // 40 bougies ascendantes avec oscillation.
        (0..40)
            .map(|i| {
                let o = 100.0 + i as f64;
                let c = o + 1.5;
                (0, o, c + 0.7, o - 0.8, c)
            })
            .collect()
    }

    #[test]
    fn rend_un_png_significatif() {
        let p = ParamsCapture { bougies: bougies(), zone_haut: 130.0, zone_bas: 125.0, achat: true, prix: 124.0 };
        let png = rendre(&p).expect("png");
        assert!(png.len() > 5_000, "image non triviale");
        assert_eq!(&png[1..4], b"PNG", "signature PNG");
    }

    #[test]
    fn trop_peu_de_bougies_rend_none() {
        let p = ParamsCapture { bougies: vec![(0, 1.0, 1.0, 1.0, 1.0)], zone_haut: 1.0, zone_bas: 1.0, achat: true, prix: 1.0 };
        assert!(rendre(&p).is_none());
    }

    #[test]
    fn zone_englobe_les_pixels_d_attente() {
        // La bande translucide ne doit écraser ni la signature ni le fond
        // hors zone : deux rendus (zone haute vs basse) diffèrent.
        let base = ParamsCapture { bougies: bougies(), zone_haut: 130.0, zone_bas: 125.0, achat: true, prix: 124.0 };
        let autre = ParamsCapture { zone_haut: 110.0, zone_bas: 105.0, ..base.clone() };
        let a = rendre(&base).expect("a");
        let b = rendre(&autre).expect("b");
        assert_ne!(a, b);
    }
}
