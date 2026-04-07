/// SVG pré-construits pour les concepts SMC courants.
/// Retourne None si le concept n'est pas dans la bibliothèque → fallback LLM.
pub fn trouver_template(sujet: &str) -> Option<&'static str> {
    let s = sujet.to_lowercase();
    if s.contains("ifvg") || s.contains("inversion fair") {
        return Some(SVG_IFVG);
    }
    if (s.contains("fvg") || s.contains("fair value gap")) && !s.contains("ifvg") {
        return Some(SVG_FVG);
    }
    if s.contains("order block")
        || s.contains("order_block")
        || (s.contains(" ob") && !s.contains("bos"))
        || s.starts_with("ob ")
    {
        return Some(SVG_ORDER_BLOCK);
    }
    if s.contains("bos")
        || s.contains("break of structure")
        || s.contains("choch")
        || s.contains("change of character")
    {
        return Some(SVG_BOS_CHOCH);
    }
    None
}

const SVG_IFVG: &str = r####"<svg width="100%" viewBox="0 0 580 280" xmlns="http://www.w3.org/2000/svg" font-family="Inter,sans-serif">
<rect width="580" height="280" fill="#0d1117"/>
<defs><marker id="a" markerWidth="8" markerHeight="8" refX="8" refY="4" orient="auto"><path d="M0,0L8,4L0,8Z" fill="#22c55e"/></marker></defs>
<text x="290" y="22" text-anchor="middle" font-size="13" font-weight="600" fill="#e6edf3">IFVG — Inversion Fair Value Gap (Haussier)</text>
<rect x="168" y="112" width="382" height="56" fill="#8b5cf6" fill-opacity="0.12" stroke="#8b5cf6" stroke-width="1"/>
<line x1="50" y1="168" x2="555" y2="168" stroke="#8b5cf6" stroke-width="0.8" stroke-dasharray="3,4" stroke-opacity="0.7"/>
<line x1="168" y1="112" x2="555" y2="112" stroke="#8b5cf6" stroke-width="0.8" stroke-dasharray="3,4" stroke-opacity="0.7"/>
<text x="556" y="116" font-size="9" fill="#8b5cf6">C3 low</text>
<text x="556" y="172" font-size="9" fill="#8b5cf6">C1 high</text>
<text x="335" y="144" text-anchor="middle" font-size="9" fill="#8b5cf6">FVG → IFVG (support apres rejet)</text>
<line x1="60" y1="165" x2="60" y2="206" stroke="#ef4444" stroke-width="1.5"/>
<rect x="51" y="175" width="18" height="22" fill="#ef4444" rx="1"/>
<line x1="110" y1="82" x2="110" y2="210" stroke="#22c55e" stroke-width="1.5"/>
<rect x="101" y="87" width="18" height="113" fill="#22c55e" rx="1"/>
<line x1="160" y1="72" x2="160" y2="112" stroke="#22c55e" stroke-width="1.5"/>
<rect x="151" y="75" width="18" height="12" fill="#22c55e" rx="1"/>
<text x="108" y="255" text-anchor="middle" font-size="9" fill="#8b949e">1. Impulsion + FVG cree</text>
<line x1="250" y1="70" x2="250" y2="135" stroke="#ef4444" stroke-width="1.5"/>
<rect x="241" y="75" width="18" height="52" fill="#ef4444" rx="1"/>
<line x1="300" y1="128" x2="300" y2="163" stroke="#ef4444" stroke-width="1.5"/>
<rect x="291" y="133" width="18" height="26" fill="#ef4444" rx="1"/>
<text x="275" y="271" text-anchor="middle" font-size="9" fill="#8b949e">2. Retest zone</text>
<line x1="350" y1="100" x2="350" y2="166" stroke="#22c55e" stroke-width="1.5"/>
<rect x="341" y="108" width="18" height="50" fill="#22c55e" rx="1"/>
<line x1="350" y1="100" x2="350" y2="78" stroke="#22c55e" stroke-width="2" marker-end="url(#a)"/>
<text x="350" y="255" text-anchor="middle" font-size="9" fill="#22c55e">3. Rejet IFVG confirme</text>
<line x1="400" y1="78" x2="400" y2="112" stroke="#22c55e" stroke-width="1.5"/>
<rect x="391" y="82" width="18" height="26" fill="#22c55e" rx="1"/>
<line x1="450" y1="56" x2="450" y2="84" stroke="#22c55e" stroke-width="1.5"/>
<rect x="441" y="60" width="18" height="22" fill="#22c55e" rx="1"/>
<text x="425" y="271" text-anchor="middle" font-size="9" fill="#8b949e">4. Continuation haussiere</text>
</svg>"####;

const SVG_FVG: &str = r####"<svg width="100%" viewBox="0 0 580 280" xmlns="http://www.w3.org/2000/svg" font-family="Inter,sans-serif">
<rect width="580" height="280" fill="#0d1117"/>
<text x="290" y="22" text-anchor="middle" font-size="13" font-weight="600" fill="#e6edf3">FVG — Fair Value Gap (Haussier)</text>
<rect x="175" y="115" width="370" height="50" fill="#3b82f6" fill-opacity="0.15" stroke="#3b82f6" stroke-width="1"/>
<line x1="50" y1="165" x2="550" y2="165" stroke="#3b82f6" stroke-width="0.8" stroke-dasharray="3,4" stroke-opacity="0.7"/>
<line x1="175" y1="115" x2="550" y2="115" stroke="#3b82f6" stroke-width="0.8" stroke-dasharray="3,4" stroke-opacity="0.7"/>
<text x="555" y="119" font-size="9" fill="#3b82f6">C3 low</text>
<text x="555" y="169" font-size="9" fill="#3b82f6">C1 high</text>
<line x1="155" y1="115" x2="155" y2="165" stroke="#f59e0b" stroke-width="2"/>
<text x="148" y="144" text-anchor="end" font-size="9" fill="#f59e0b">GAP</text>
<text x="350" y="144" text-anchor="middle" font-size="9" fill="#3b82f6">Zone d inefficacite — prix tend a combler</text>
<line x1="70" y1="162" x2="70" y2="202" stroke="#ef4444" stroke-width="1.5"/>
<rect x="61" y="172" width="18" height="22" fill="#ef4444" rx="1"/>
<line x1="120" y1="78" x2="120" y2="205" stroke="#22c55e" stroke-width="1.5"/>
<rect x="111" y="85" width="18" height="115" fill="#22c55e" rx="1"/>
<line x1="175" y1="65" x2="175" y2="115" stroke="#22c55e" stroke-width="1.5"/>
<rect x="166" y="70" width="18" height="40" fill="#22c55e" rx="1"/>
<line x1="230" y1="52" x2="230" y2="75" stroke="#22c55e" stroke-width="1.5"/>
<rect x="221" y="56" width="18" height="16" fill="#22c55e" rx="1"/>
<line x1="285" y1="44" x2="285" y2="58" stroke="#22c55e" stroke-width="1.5"/>
<rect x="276" y="48" width="18" height="8" fill="#22c55e" rx="1"/>
<line x1="360" y1="44" x2="360" y2="100" stroke="#ef4444" stroke-width="1.5"/>
<rect x="351" y="54" width="18" height="40" fill="#ef4444" rx="1"/>
<line x1="415" y1="95" x2="415" y2="140" stroke="#ef4444" stroke-width="1.5"/>
<rect x="406" y="102" width="18" height="32" fill="#ef4444" rx="1"/>
<line x1="415" y1="142" x2="415" y2="160" stroke="#f59e0b" stroke-width="2" stroke-dasharray="3,2"/>
<text x="440" y="158" font-size="9" fill="#f59e0b">vers FVG</text>
<text x="175" y="272" text-anchor="middle" font-size="9" fill="#8b949e">1. FVG cree (3 bougies, gap = inefficacite)</text>
<text x="415" y="272" text-anchor="middle" font-size="9" fill="#8b949e">2. Prix cherche a combler</text>
</svg>"####;

const SVG_ORDER_BLOCK: &str = r####"<svg width="100%" viewBox="0 0 580 280" xmlns="http://www.w3.org/2000/svg" font-family="Inter,sans-serif">
<rect width="580" height="280" fill="#0d1117"/>
<defs><marker id="a" markerWidth="8" markerHeight="8" refX="8" refY="4" orient="auto"><path d="M0,0L8,4L0,8Z" fill="#22c55e"/></marker></defs>
<text x="290" y="22" text-anchor="middle" font-size="13" font-weight="600" fill="#e6edf3">Order Block (OB) Haussier</text>
<rect x="148" y="148" width="400" height="42" fill="#f59e0b" fill-opacity="0.12" stroke="#f59e0b" stroke-width="1"/>
<line x1="40" y1="148" x2="555" y2="148" stroke="#f59e0b" stroke-width="0.8" stroke-dasharray="3,4" stroke-opacity="0.7"/>
<line x1="40" y1="190" x2="555" y2="190" stroke="#f59e0b" stroke-width="0.8" stroke-dasharray="3,4" stroke-opacity="0.7"/>
<text x="320" y="173" text-anchor="middle" font-size="9" fill="#f59e0b">Zone Order Block (support institutionnel)</text>
<line x1="55" y1="170" x2="55" y2="210" stroke="#22c55e" stroke-width="1.5"/>
<rect x="46" y="178" width="18" height="20" fill="#22c55e" rx="1"/>
<line x1="100" y1="163" x2="100" y2="205" stroke="#ef4444" stroke-width="1.5"/>
<rect x="91" y="172" width="18" height="24" fill="#ef4444" rx="1"/>
<line x1="148" y1="142" x2="148" y2="198" stroke="#ef4444" stroke-width="1.5"/>
<rect x="139" y="148" width="18" height="42" fill="#ef4444" rx="1" stroke="#f59e0b" stroke-width="2"/>
<text x="148" y="136" text-anchor="middle" font-size="9" fill="#f59e0b" font-weight="600">OB</text>
<line x1="200" y1="68" x2="200" y2="202" stroke="#22c55e" stroke-width="1.5"/>
<rect x="191" y="75" width="18" height="122" fill="#22c55e" rx="1"/>
<line x1="255" y1="55" x2="255" y2="85" stroke="#22c55e" stroke-width="1.5"/>
<rect x="246" y="60" width="18" height="22" fill="#22c55e" rx="1"/>
<line x1="305" y1="45" x2="305" y2="66" stroke="#22c55e" stroke-width="1.5"/>
<rect x="296" y="50" width="18" height="13" fill="#22c55e" rx="1"/>
<line x1="365" y1="45" x2="365" y2="108" stroke="#ef4444" stroke-width="1.5"/>
<rect x="356" y="55" width="18" height="48" fill="#ef4444" rx="1"/>
<line x1="415" y1="100" x2="415" y2="162" stroke="#ef4444" stroke-width="1.5"/>
<rect x="406" y="108" width="18" height="48" fill="#ef4444" rx="1"/>
<line x1="465" y1="78" x2="465" y2="190" stroke="#22c55e" stroke-width="1.5"/>
<rect x="456" y="83" width="18" height="98" fill="#22c55e" rx="1"/>
<line x1="465" y1="78" x2="465" y2="60" stroke="#22c55e" stroke-width="2" marker-end="url(#a)"/>
<text x="148" y="255" text-anchor="middle" font-size="9" fill="#f59e0b">1. Dernier bearish = OB</text>
<text x="255" y="271" text-anchor="middle" font-size="9" fill="#8b949e">2. Impulsion</text>
<text x="415" y="255" text-anchor="middle" font-size="9" fill="#8b949e">3. Retest OB</text>
<text x="465" y="271" text-anchor="middle" font-size="9" fill="#22c55e">4. Support confirme</text>
</svg>"####;

const SVG_BOS_CHOCH: &str = r####"<svg width="100%" viewBox="0 0 580 280" xmlns="http://www.w3.org/2000/svg" font-family="Inter,sans-serif">
<rect width="580" height="280" fill="#0d1117"/>
<text x="290" y="22" text-anchor="middle" font-size="13" font-weight="600" fill="#e6edf3">BOS et CHoCH — Structures de Marche</text>
<line x1="40" y1="108" x2="560" y2="108" stroke="#f59e0b" stroke-width="1" stroke-dasharray="5,4" stroke-opacity="0.8"/>
<text x="562" y="112" font-size="8" fill="#f59e0b">LH1</text>
<line x1="400" y1="35" x2="400" y2="255" stroke="#22c55e" stroke-width="1.5" stroke-dasharray="4,3"/>
<text x="400" y="260" text-anchor="middle" font-size="9" fill="#22c55e" font-weight="600">CHoCH</text>
<polyline points="40,75 100,200 160,108 220,215 280,118 340,228 400,95" fill="none" stroke="#ef4444" stroke-width="2.5"/>
<polyline points="400,95 460,185 510,70 550,140" fill="none" stroke="#22c55e" stroke-width="2.5"/>
<circle cx="40" cy="75" r="4" fill="#8b949e"/>
<circle cx="100" cy="200" r="4" fill="#ef4444"/>
<text x="100" y="215" text-anchor="middle" font-size="9" fill="#ef4444">LL1</text>
<circle cx="160" cy="108" r="4" fill="#ef4444"/>
<text x="160" y="100" text-anchor="middle" font-size="9" fill="#ef4444">LH1</text>
<circle cx="220" cy="215" r="4" fill="#ef4444"/>
<text x="220" y="228" text-anchor="middle" font-size="9" fill="#ef4444">LL2</text>
<circle cx="280" cy="118" r="4" fill="#ef4444"/>
<text x="280" y="110" text-anchor="middle" font-size="9" fill="#ef4444">LH2</text>
<circle cx="340" cy="228" r="4" fill="#ef4444"/>
<text x="340" y="242" text-anchor="middle" font-size="9" fill="#ef4444">LL3</text>
<circle cx="400" cy="95" r="5" fill="#22c55e" stroke="#e6edf3" stroke-width="1.5"/>
<text x="420" y="88" font-size="9" fill="#22c55e" font-weight="600">Break LH1</text>
<circle cx="460" cy="185" r="4" fill="#22c55e"/>
<text x="460" y="200" text-anchor="middle" font-size="9" fill="#22c55e">HL1</text>
<circle cx="510" cy="70" r="4" fill="#22c55e"/>
<text x="510" y="62" text-anchor="middle" font-size="9" fill="#22c55e">HH1</text>
<text x="200" y="270" text-anchor="middle" font-size="9" fill="#ef4444">Downtrend : LH + LL (BOS baissier)</text>
<text x="480" y="270" text-anchor="middle" font-size="9" fill="#22c55e">Uptrend : HH + HL</text>
</svg>"####;
