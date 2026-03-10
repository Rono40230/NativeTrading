import { useState, useCallback, useRef, useEffect } from "react";


const ANALYST_PROMPT = `Tu es un analyste expert en Smart Money Concepts (SMC) appliqués au Bitcoin et aux cryptomonnaies. Ton rôle est d'analyser les graphiques fournis avec une précision institutionnelle.

=== MÉTHODOLOGIE D'ANALYSE ===

Analyse la capture d'écran fournie selon les critères SMC suivants :

1. STRUCTURE DE MARCHÉ :
   - Biais directionnel (Bullish / Bearish / Neutre)
   - Phase de tendance (Accumulation / Tendance / Distribution / Retournement)
   - Dernier événement structurel (BOS / ChoCH / Aucun)
   - Identification des HH, HL, LH, LL
   - Niveaux clés

2. LIQUIDITÉ :
   - Zones BSL et SSL
   - Niveaux d'inducement
   - Sweep récent de liquidité

3. POINTS D'INTÉRÊT (POI) :
   - Order Blocks (Demand / Supply) avec force
   - Fair Value Gaps (FVG)
   - Breaker Blocks
   - Zones de réaction probables

4. OBJECTIFS DE PRIX :
   - Cibles haussières / baissières
   - Niveau d'invalidation

5. CONFLUENCE

=== FORMAT DE RÉPONSE ===

**📊 VUE D'ENSEMBLE**
Résumé en 2-3 phrases.

**🏗️ STRUCTURE DE MARCHÉ**

**💧 ANALYSE DE LIQUIDITÉ**

**🎯 POINTS D'INTÉRÊT**

**📈 SCÉNARIO PRINCIPAL** (probabilité estimée)

**📉 SCÉNARIO ALTERNATIF** (probabilité estimée)

**⚡ NIVEAUX À SURVEILLER**

**🔑 CONCLUSION** + Confiance /10

=== RÈGLES ===
1. Base-toi UNIQUEMENT sur le graphique visible.
2. Si flou/illisible, dis-le.
3. Ne fabrique pas de niveaux de prix.
4. Confiance globale sur 10.`;

const COACH_PROMPT = `Tu es un coach expert en Smart Money Concepts (SMC). Tu enseignes la méthodologie SMC de manière claire, visuelle et pédagogique. Tu parles TOUJOURS en français.

=== TON RÔLE ===
- Répondre aux questions sur les concepts SMC (Order Blocks, FVG, liquidité, BOS, ChoCH, etc.)
- Quand on te demande d'expliquer un concept visuellement ou de faire un dessin/schéma, tu DOIS produire un diagramme HTML interactif
- Quand on te montre un screenshot de trade, analyse-le et explique POURQUOI la zone est bonne ou mauvaise
- Adapte-toi au niveau du trader

=== DIAGRAMMES HTML INTERACTIFS ===

C'est CRUCIAL. Quand tu dois illustrer un concept visuellement, tu produis un bloc HTML complet enveloppé dans des balises <htmldiagram>...</htmldiagram>.

Le HTML doit être un document COMPLET et autonome avec <style> et <script> intégrés. Utilise SVG ou des divs positionnés pour dessiner. Ajoute des animations CSS, des effets hover, ou des éléments cliquables pour rendre le schéma dynamique et interactif.

Règles pour les diagrammes :
- Fond TOUJOURS sombre : #0d1117 ou similaire
- Couleurs principales : #63b3ed (bleu), #b794f4 (violet), #22c55e (vert haussier), #ef4444 (rouge baissier), #f59e0b (jaune alerte)
- Texte : #e6edf3 (clair) ou #8b949e (secondaire)
- Polices : font-family: 'Inter', -apple-system, sans-serif
- Le diagramme doit faire max 680px de large et s'adapter
- Ajoute des labels, flèches, annotations
- Utilise des animations CSS (pulse, fadeIn, mouvement) pour rendre le schéma vivant
- Ajoute des tooltips au hover quand c'est pertinent
- Si possible, rends des éléments cliquables pour montrer plus d'infos
- Le HTML doit être 100% autonome (pas de dépendances externes)
- Ajoute toujours un titre en haut du diagramme

Exemples de concepts à illustrer avec des diagrammes :
- Order Block : montrer les bougies, la zone OB colorée, le mouvement impulsif avec animation
- FVG : 3 bougies avec le gap mis en évidence, animation de "fill"
- Liquidité : niveaux avec clusters de stop-loss, sweep animé
- BOS / ChoCH : structure de marché avec les HH/HL/LH/LL connectés
- Structure de marché complète : tendance avec annotations

=== QUAND ON TE MONTRE UN TRADE ===

Analyse le screenshot et réponds :
1. **✅ Ce qui est bien** — les confluences SMC présentes
2. **⚠️ Ce qui manque** — les éléments non vérifiés
3. **💡 Conseil** — comment améliorer l'entrée
4. **Note** — /10 pour la qualité du setup

=== RÈGLES ===
- Sois concis et direct
- Utilise des émojis pour structurer
- Produis un <htmldiagram> chaque fois qu'une illustration visuelle aiderait la compréhension
- N'invente jamais de niveaux de prix sur les screenshots
- Encourage le trader tout en étant rigoureux
- Parle TOUJOURS en français`;

export default function SMCAnalyzer() {
  const [mode, setMode] = useState("analyst");
  const [image, setImage] = useState(null);
  const [preview, setPreview] = useState(null);
  const [dragOver, setDragOver] = useState(false);
  const [notes, setNotes] = useState("");
  const [analyzing, setAnalyzing] = useState(false);
  const [result, setResult] = useState(null);
  const [error, setError] = useState(null);
  const [messages, setMessages] = useState([]);
  const [chatInput, setChatInput] = useState("");
  const [chatImage, setChatImage] = useState(null);
  const [chatPreview, setChatPreview] = useState(null);
  const [chatLoading, setChatLoading] = useState(false);
  const chatEndRef = useRef(null);

  useEffect(() => { chatEndRef.current?.scrollIntoView({ behavior: "smooth" }); }, [messages, chatLoading]);

  // === MARKDOWN RENDERER ===
  const renderMd = (text) => {
    return text
      .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
      .replace(/```([\s\S]*?)```/g, (m, code) =>
        `<pre style="background:#161b22;border:1px solid #21262d;border-radius:8px;padding:12px;overflow-x:auto;font-family:monospace;font-size:12px;line-height:1.4;color:#7ee787;margin:6px 0">${code.trim()}</pre>`)
      .replace(/`([^`]+)`/g, '<code style="background:#161b22;padding:2px 6px;border-radius:4px;font-size:12px;color:#7ee787">$1</code>')
      .replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>")
      .replace(/\*\*(.+?)\*\*/g, "<strong style='color:#e6edf3'>$1</strong>")
      .replace(/\*(.+?)\*/g, "<em>$1</em>")
      .replace(/^### (.+)$/gm, "<h3 style='font-size:14px;color:#b794f4;margin:8px 0 2px'>$1</h3>")
      .replace(/^## (.+)$/gm, "<h2 style='font-size:15px;color:#63b3ed;margin:10px 0 2px'>$1</h2>")
      .replace(/^# (.+)$/gm, "<h1 style='font-size:17px;color:#e6edf3;margin:12px 0 4px'>$1</h1>")
      .replace(/^---$/gm, "<hr style='border:none;border-top:1px solid #21262d;margin:8px 0'/>")
      .replace(/^- (.+)$/gm, "<div style='display:flex;gap:8px;margin:1px 0'><span style='color:#63b3ed'>•</span><span>$1</span></div>")
      .replace(/^\d+\. (.+)$/gm, (m, p1) => {
        const n = m.match(/^(\d+)/)[1];
        return `<div style='display:flex;gap:8px;margin:1px 0'><span style='color:#63b3ed;min-width:18px'>${n}.</span><span>${p1}</span></div>`;
      })
      .replace(/\n\n/g, "<br/>")
      .replace(/\n/g, "<br/>");
  };

  // === PARSE DIAGRAMS FROM RESPONSE ===
  const parseContent = (text) => {
    const parts = [];
    const regex = /<htmldiagram>([\s\S]*?)<\/htmldiagram>/g;
    let last = 0, match;
    while ((match = regex.exec(text)) !== null) {
      if (match.index > last) parts.push({ type: "text", content: text.slice(last, match.index) });
      parts.push({ type: "diagram", content: match[1] });
      last = regex.lastIndex;
    }
    if (last < text.length) parts.push({ type: "text", content: text.slice(last) });
    return parts;
  };

  // === ANALYST FILE HANDLING ===
  const handleFile = useCallback((file) => {
    if (!file || !file.type.startsWith("image/")) return;
    setImage(file); setResult(null); setError(null);
    const r = new FileReader();
    r.onload = (e) => setPreview(e.target.result);
    r.readAsDataURL(file);
  }, []);

  const resetAnalyst = () => { setImage(null); setPreview(null); setNotes(""); setResult(null); setError(null); };

  const analyze = async () => {
    if (!preview) return;
    setAnalyzing(true); setError(null); setResult(null);
    const msg = notes ? `Analyse ce graphique selon la méthodologie SMC.\n\nNotes : ${notes}` : "Analyse ce graphique selon la méthodologie SMC.";
    try {
      const res = await fetch("https://api.anthropic.com/v1/messages", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model: "claude-opus-4-6", max_tokens: 4000, system: ANALYST_PROMPT,
          messages: [{ role: "user", content: [
            { type: "image", source: { type: "base64", media_type: image.type || "image/png", data: preview.split(",")[1] } },
            { type: "text", text: msg }
          ]}]
        })
      });
      const data = await res.json();
      if (data.error) setError(data.error.message);
      else setResult(data.content?.map(b => b.text || "").join("\n") || "Aucune réponse.");
    } catch (err) { setError("Erreur : " + err.message); }
    finally { setAnalyzing(false); }
  };

  // === COACH ===
  const handleChatImage = (file) => {
    if (!file || !file.type.startsWith("image/")) return;
    setChatImage(file);
    const r = new FileReader();
    r.onload = (e) => setChatPreview(e.target.result);
    r.readAsDataURL(file);
  };

  const sendChat = async () => {
    if (!chatInput.trim() && !chatPreview) return;
    setChatLoading(true);
    const displayImg = chatPreview || null;
    const newUserMsg = { role: "user", text: chatInput, image: displayImg };
    const updated = [...messages, newUserMsg];
    setMessages(updated);
    setChatInput(""); setChatImage(null); setChatPreview(null);

    const apiMsgs = updated.map(m => {
      if (m.role === "user") {
        const c = [];
        if (m.image) c.push({ type: "image", source: { type: "base64", media_type: "image/png", data: m.image.split(",")[1] } });
        c.push({ type: "text", text: m.text || "Analyse ce screenshot." });
        return { role: "user", content: c };
      }
      return { role: "assistant", content: m.text };
    }).slice(-10);

    try {
      const res = await fetch("https://api.anthropic.com/v1/messages", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model: "claude-opus-4-6", max_tokens: 4000, system: COACH_PROMPT, messages: apiMsgs })
      });
      const data = await res.json();
      const reply = data.error ? `⚠️ ${data.error.message}` : (data.content?.map(b => b.text || "").join("\n") || "…");
      setMessages(prev => [...prev, { role: "assistant", text: reply }]);
    } catch (err) {
      setMessages(prev => [...prev, { role: "assistant", text: `⚠️ Erreur : ${err.message}` }]);
    } finally { setChatLoading(false); }
  };

  const onKeyDown = (e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendChat(); } };

  return (
    <div style={{ minHeight: "100vh", background: "linear-gradient(135deg, #0a0a0f 0%, #0d1117 50%, #0a0f1a 100%)", color: "#e6edf3", fontFamily: "'Inter', -apple-system, sans-serif", padding: 20 }}>
      <div style={{ maxWidth: 720, margin: "0 auto" }}>
        <div style={{ marginBottom: 16, position: "relative" }}>
          <div style={{ position: "absolute", left: 0, top: 0 }}>
            <svg width="36" height="36" viewBox="0 0 80 80">
              <defs>
                <linearGradient id="dropGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#63b3ed"/>
                  <stop offset="100%" stopColor="#3cee91"/>
                </linearGradient>
                <filter id="dropGlow"><feGaussianBlur stdDeviation="2" result="blur"/><feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
              </defs>
              <path d="M40 6 C40 6 14 38 14 53 C14 67.4 25.6 78 40 78 C54.4 78 66 67.4 66 53 C66 38 40 6 40 6Z" fill="url(#dropGrad)" opacity="0.1"/>
              <path d="M40 6 C40 6 14 38 14 53 C14 67.4 25.6 78 40 78 C54.4 78 66 67.4 66 53 C66 38 40 6 40 6Z" fill="none" stroke="url(#dropGrad)" strokeWidth="2.5" filter="url(#dropGlow)"/>
            </svg>
          </div>
          <div style={{ textAlign: "center" }}>
            <h1 style={{ fontSize: 22, fontWeight: 700, margin: 0, color: "#3cee91" }}>Le petit robo</h1>
            <p style={{ color: "#8b949e", fontSize: 12, margin: 0 }}>Smart Money Concepts</p>
          </div>
        </div>

        {/* Mode Toggle */}
        <div style={{ display: "flex", background: "#161b22", borderRadius: 10, border: "1px solid #21262d", padding: 4, marginBottom: 20, gap: 4 }}>
          {[{ id: "analyst", icon: "🔍", label: "Analyste" }, { id: "coach", icon: "🎓", label: "Coach" }].map(t => (
            <button key={t.id} onClick={() => setMode(t.id)} style={{
              flex: 1, padding: "10px 16px", borderRadius: 8, border: "none", fontSize: 14, fontWeight: 600, cursor: "pointer", transition: "all 0.2s",
              background: mode === t.id ? "linear-gradient(90deg, #63b3ed22, #b794f422)" : "transparent",
              color: mode === t.id ? "#e6edf3" : "#484f58"
            }}>{t.icon} {t.label}</button>
          ))}
        </div>

        {/* ===================== ANALYST ===================== */}
        {mode === "analyst" && (
          <>
            <Box icon="🖼️" title="Dépose ta capture d'écran">
              <p style={{ color: "#8b949e", fontSize: 13, margin: "0 0 12px" }}>Glisse ou sélectionne la capture de ton graphique</p>
              {!preview ? (
                <DropZone dragOver={dragOver}
                  onDrop={e => { e.preventDefault(); setDragOver(false); handleFile(e.dataTransfer.files[0]); }}
                  onDragOver={e => { e.preventDefault(); setDragOver(true); }}
                  onDragLeave={() => setDragOver(false)}
                  onClick={() => document.getElementById("f-a").click()}>
                  <input id="f-a" type="file" accept="image/*" style={{ display: "none" }} onChange={e => handleFile(e.target.files[0])} />
                </DropZone>
              ) : (
                <div>
                  <div style={{ borderRadius: 10, overflow: "hidden", border: "1px solid #21262d", marginBottom: 12 }}>
                    <img src={preview} alt="" style={{ width: "100%", display: "block" }} />
                  </div>
                  <div style={{ display: "flex", gap: 8 }}>
                    <BtnSec onClick={resetAnalyst}>🗑️ Supprimer</BtnSec>
                    <BtnSec onClick={() => document.getElementById("f-r").click()}>🔄 Remplacer</BtnSec>
                    <input id="f-r" type="file" accept="image/*" style={{ display: "none" }} onChange={e => handleFile(e.target.files[0])} />
                  </div>
                </div>
              )}
              <div style={{ marginTop: 14 }}>
                <label style={{ display: "block", fontSize: 13, color: "#8b949e", marginBottom: 6 }}>Notes (optionnel) :</label>
                <textarea value={notes} onChange={e => setNotes(e.target.value)} placeholder="Ex : Timeframe 1H, contexte macro…" style={taStyle} />
              </div>
              <button onClick={analyze} disabled={!preview || analyzing} style={{
                ...btnP, marginTop: 14,
                background: preview && !analyzing ? "linear-gradient(90deg, #63b3ed, #b794f4)" : "#21262d",
                color: preview && !analyzing ? "#fff" : "#484f58",
                cursor: preview && !analyzing ? "pointer" : "not-allowed"
              }}>{analyzing ? "⏳ Analyse en cours…" : "🔍 Analyser le graphique"}</button>
            </Box>

            {(result || error) && (
              <Box icon="📈" title="Résultat">
                {error
                  ? <div style={{ background: "rgba(239,68,68,0.08)", border: "1px solid rgba(239,68,68,0.2)", borderRadius: 8, padding: 14, color: "#f87171", fontSize: 13 }}>⚠️ {error}</div>
                  : <RichContent text={result} renderMd={renderMd} parseContent={parseContent} />
                }
              </Box>
            )}
          </>
        )}

        {/* ===================== COACH ===================== */}
        {mode === "coach" && (
          <Box icon="🎓" title="Coach SMC">
            <p style={{ color: "#8b949e", fontSize: 13, margin: "0 0 16px" }}>
              Pose tes questions sur la SMC, demande des schémas visuels, ou envoie tes screenshots de trades.
            </p>

            <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 16 }}>
              {[
                "C'est quoi un Order Block ? Fais-moi un schéma",
                "Explique-moi la liquidité avec un dessin",
                "C'est quoi un FVG ? Montre-moi",
                "BOS vs ChoCH ? Illustre",
                "Montre-moi un setup parfait"
              ].map((q, i) => (
                <button key={i} onClick={() => setChatInput(q)} style={{
                  background: "#161b22", border: "1px solid #21262d", color: "#8b949e",
                  padding: "6px 12px", borderRadius: 20, fontSize: 12, cursor: "pointer"
                }}>{q}</button>
              ))}
            </div>

            <div style={{ background: "#0d1117", border: "1px solid #21262d", borderRadius: 10, minHeight: 300, maxHeight: 600, overflow: "auto", padding: 14, marginBottom: 12 }}>
              {messages.length === 0 && (
                <div style={{ textAlign: "center", color: "#30363d", paddingTop: 80 }}>
                  <div style={{ fontSize: 32, marginBottom: 8 }}>🎓</div>
                  <p style={{ fontSize: 13 }}>Pose ta première question…</p>
                </div>
              )}
              {messages.map((m, i) => (
                <div key={i} style={{ display: "flex", flexDirection: "column", alignItems: m.role === "user" ? "flex-end" : "flex-start", marginBottom: 14 }}>
                  <div style={{ fontSize: 10, color: "#484f58", marginBottom: 4 }}>{m.role === "user" ? "Toi" : "🎓 Coach"}</div>
                  {m.image && <img src={m.image} alt="" style={{ maxWidth: 200, borderRadius: 8, border: "1px solid #21262d", marginBottom: 6 }} />}
                  <div style={{
                    background: m.role === "user" ? "rgba(99,179,237,0.12)" : "#161b22",
                    border: `1px solid ${m.role === "user" ? "rgba(99,179,237,0.2)" : "#21262d"}`,
                    borderRadius: 10, padding: "10px 14px", maxWidth: m.role === "assistant" ? "100%" : "85%",
                    fontSize: 13, lineHeight: 1.45, color: "#c9d1d9"
                  }}>
                    {m.role === "user"
                      ? <span>{m.text}</span>
                      : <RichContent text={m.text} renderMd={renderMd} parseContent={parseContent} />
                    }
                  </div>
                </div>
              ))}
              {chatLoading && <div style={{ color: "#484f58", fontSize: 13 }}>🎓 Réflexion en cours…</div>}
              <div ref={chatEndRef} />
            </div>

            {chatPreview && (
              <div style={{ marginBottom: 8, display: "flex", alignItems: "center", gap: 8 }}>
                <img src={chatPreview} alt="" style={{ height: 48, borderRadius: 6, border: "1px solid #21262d" }} />
                <button onClick={() => { setChatImage(null); setChatPreview(null); }} style={{ background: "transparent", border: "none", color: "#f87171", fontSize: 16, cursor: "pointer" }}>✕</button>
              </div>
            )}

            <div style={{ display: "flex", gap: 8, alignItems: "flex-end" }}>
              <button onClick={() => document.getElementById("f-c").click()} style={{
                background: "#161b22", border: "1px solid #21262d", color: "#8b949e",
                width: 40, height: 40, borderRadius: 8, fontSize: 16, cursor: "pointer",
                display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0
              }}>📎</button>
              <input id="f-c" type="file" accept="image/*" style={{ display: "none" }} onChange={e => { handleChatImage(e.target.files[0]); e.target.value = ""; }} />
              <textarea value={chatInput} onChange={e => setChatInput(e.target.value)} onKeyDown={onKeyDown}
                placeholder="Pose ta question ou envoie un screenshot…" rows={1}
                style={{ ...taStyle, minHeight: 40, maxHeight: 100, flex: 1, margin: 0, resize: "none", padding: "10px 12px" }} />
              <button onClick={sendChat} disabled={chatLoading || (!chatInput.trim() && !chatPreview)} style={{
                ...btnP, width: 40, height: 40, padding: 0, flexShrink: 0,
                display: "flex", alignItems: "center", justifyContent: "center",
                background: (!chatLoading && (chatInput.trim() || chatPreview)) ? "linear-gradient(90deg, #63b3ed, #b794f4)" : "#21262d",
                color: (!chatLoading && (chatInput.trim() || chatPreview)) ? "#fff" : "#484f58",
                cursor: (!chatLoading && (chatInput.trim() || chatPreview)) ? "pointer" : "not-allowed"
              }}>➤</button>
            </div>

            {messages.length > 0 && (
              <button onClick={() => setMessages([])} style={{ background: "transparent", border: "none", color: "#484f58", fontSize: 12, cursor: "pointer", marginTop: 10, padding: 0 }}>
                🗑️ Effacer la conversation
              </button>
            )}
          </Box>
        )}

        <Box icon="✅" title="Checklist — Bonne capture">
          <div style={{ display: "grid", gap: 5 }}>
            {["Timeframe visible", "Échelle de prix lisible", "50-100 bougies minimum", "Pas d'indicateurs superflus", "Volumes visibles (si possible)"]
              .map((t, i) => <div key={i} style={{ display: "flex", gap: 8, fontSize: 13, color: "#8b949e" }}><span style={{ color: "#22c55e" }}>✓</span>{t}</div>)}
          </div>
        </Box>

        <p style={{ textAlign: "center", color: "#30363d", fontSize: 11, marginTop: 28 }}>SMC IA v2.1 — Opus 4.6</p>
      </div>
    </div>
  );
}

// === RICH CONTENT: text + embedded HTML diagrams ===
function RichContent({ text, renderMd, parseContent }) {
  const parts = parseContent(text);
  return (
    <div>
      {parts.map((p, i) =>
        p.type === "text"
          ? <div key={i} style={{ fontSize: 13.5, lineHeight: 1.45, color: "#c9d1d9" }} dangerouslySetInnerHTML={{ __html: renderMd(p.content) }} />
          : <DiagramFrame key={i} html={p.content} />
      )}
    </div>
  );
}

// === IFRAME SANDBOX FOR HTML DIAGRAMS ===
function DiagramFrame({ html }) {
  const ref = useRef(null);
  const [h, setH] = useState(400);

  useEffect(() => {
    const iframe = ref.current;
    if (!iframe) return;

    const fullHtml = `<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>*{margin:0;padding:0;box-sizing:border-box}body{background:#0d1117;color:#e6edf3;font-family:'Inter',-apple-system,sans-serif;overflow:hidden;display:flex;justify-content:center;padding:12px}</style></head><body>${html}</body><script>
      function sendHeight(){
        const h=document.body.scrollHeight;
        window.parent.postMessage({type:'resize',height:h},'*');
      }
      window.addEventListener('load',()=>{sendHeight();setTimeout(sendHeight,500);});
      new ResizeObserver(sendHeight).observe(document.body);
    <\/script></html>`;

    iframe.srcdoc = fullHtml;

    const onMsg = (e) => {
      if (e.source === iframe.contentWindow && e.data?.type === "resize") {
        setH(Math.min(Math.max(e.data.height + 24, 200), 800));
      }
    };
    window.addEventListener("message", onMsg);
    return () => window.removeEventListener("message", onMsg);
  }, [html]);

  return (
    <div style={{ margin: "12px 0", borderRadius: 10, overflow: "hidden", border: "1px solid #21262d" }}>
      <iframe ref={ref} style={{ width: "100%", height: h, border: "none", borderRadius: 10, background: "#0d1117" }}
        sandbox="allow-scripts" title="Diagramme SMC" />
    </div>
  );
}

// === SHARED ===
const taStyle = { width: "100%", background: "#0d1117", border: "1px solid #21262d", borderRadius: 8, color: "#e6edf3", padding: 10, fontSize: 13, minHeight: 60, resize: "vertical", boxSizing: "border-box" };
const btnP = { width: "100%", padding: "10px 24px", borderRadius: 8, border: "none", fontSize: 14, fontWeight: 600, transition: "all 0.2s" };

function Box({ icon, title, children }) {
  return (
    <div style={{ background: "rgba(22,27,34,0.7)", border: "1px solid #21262d", borderRadius: 12, padding: 20, marginBottom: 16 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 14 }}>
        <span style={{ fontSize: 20 }}>{icon}</span>
        <h2 style={{ fontSize: 16, fontWeight: 600, margin: 0 }}>{title}</h2>
      </div>
      {children}
    </div>
  );
}

function DropZone({ dragOver, children, ...props }) {
  return (
    <div {...props} style={{
      border: `2px dashed ${dragOver ? "#63b3ed" : "#30363d"}`, borderRadius: 12,
      padding: "44px 20px", textAlign: "center", cursor: "pointer",
      background: dragOver ? "rgba(99,179,237,0.05)" : "transparent", transition: "all 0.2s"
    }}>
      <div style={{ fontSize: 32, marginBottom: 10 }}>🖼️</div>
      <p style={{ color: "#8b949e", fontSize: 14, margin: 0 }}>Glisse ton image ici ou <span style={{ color: "#63b3ed" }}>clique pour parcourir</span></p>
      <p style={{ color: "#484f58", fontSize: 12, marginTop: 4 }}>PNG, JPG, WEBP</p>
      {children}
    </div>
  );
}

function BtnSec({ children, ...props }) {
  return <button {...props} style={{ flex: 1, background: "#21262d", border: "1px solid #30363d", color: "#e6edf3", padding: "8px 16px", borderRadius: 8, fontSize: 13, cursor: "pointer" }}>{children}</button>;
}
