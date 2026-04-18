function formaterHeures(heuresUtc) {
  if (!heuresUtc.length) return ''
  const heuresFormatteesParis = heuresUtc.sort((a, b) => a - b)
  
  const blocs = []
  let debut = heuresFormatteesParis[0]
  let fin = heuresFormatteesParis[0]

  for (let i = 1; i < heuresFormatteesParis.length; i++) {
    if (heuresFormatteesParis[i] === fin + 1) {
      fin = heuresFormatteesParis[i]
    } else {
      blocs.push(debut === fin ? `${debut}h` : `${debut}h-${fin}h`)
      debut = heuresFormatteesParis[i]
      fin = heuresFormatteesParis[i]
    }
  }
  blocs.push(debut === fin ? `${debut}h` : `${debut}h-${fin}h`)
  return blocs.join(', ') + ' Paris'
}

console.log(formaterHeures([0, 14, 15, 16]))
console.log(formaterHeures([14, 16]))
