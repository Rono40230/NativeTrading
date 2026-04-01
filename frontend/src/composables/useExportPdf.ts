import { jsPDF } from 'jspdf'
import autoTable from 'jspdf-autotable'

/** Colonnes trop larges pour le PDF — on les raccourcit pour la lisibilité */
const LABELS: Record<string, string> = {
  id:              'ID',
  asset:           'Asset',
  timeframe:       'TF',
  direction:       'Dir.',
  score:           'Score',
  prix_entree:     'Entrée',
  stop_loss:       'SL',
  tp1:             'TP1',
  tp2:             'TP2',
  tp3:             'TP3',
  strategie:       'Stratégie',
  statut:          'Statut',
  verdict:         'Verdict',
  prix_verdict:    'Px verdict',
  llm_conviction:  'Conviction',
  date_signal:     'Date',
}

function entetes(colonnes: string[]): string[] {
  return colonnes.map(c => LABELS[c] ?? c)
}

function titreStrategie(strategie: string): string {
  if (strategie === 'Rockets')         return '🚀 Export Rockets'
  if (strategie === 'Straddle')        return '⚡ Export Straddle'
  if (strategie === 'SmcDirectional')  return '🧠 Export SMC Directionnel'
  return '📊 Historique des signaux'
}

export function useExportPdf() {
  async function genererPdf(
    blob: Blob,
    separateur: string,
    nomFichier: string,
    strategie?: string,
  ): Promise<void> {
    const texte = await blob.text()
    const lignes = texte
      .replace(/^\uFEFF/, '')  // supprimer BOM
      .split('\n')
      .filter(l => l.trim())

    if (lignes.length < 2) throw new Error('Aucune donnée à exporter en PDF')

    const sep      = separateur === ';' ? ';' : ','
    const colonnes = lignes[0].split(sep)
    const corps    = lignes.slice(1).map(l => l.split(sep))

    const doc = new jsPDF({ orientation: 'landscape', unit: 'mm', format: 'a4' })

    doc.setFontSize(13)
    doc.setTextColor(16, 185, 129)
    doc.text(titreStrategie(strategie ?? ''), 14, 14)

    doc.setFontSize(8)
    doc.setTextColor(120, 120, 120)
    doc.text(`Généré le ${new Date().toLocaleString('fr-FR')}`, 14, 20)

    autoTable(doc, {
      head:               [entetes(colonnes)],
      body:               corps,
      startY:             26,
      styles:             { fontSize: 7, cellPadding: 1.5, overflow: 'ellipsize' },
      headStyles:         { fillColor: [16, 185, 129], textColor: 255, fontStyle: 'bold' },
      alternateRowStyles: { fillColor: [245, 245, 245] },
      margin:             { left: 14, right: 14 },
    })

    const pdfBlob = doc.output('blob')
    const url     = URL.createObjectURL(pdfBlob)
    const a       = document.createElement('a')
    a.href        = url
    a.download    = nomFichier
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  return { genererPdf }
}
