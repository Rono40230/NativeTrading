// Métadonnées crypto : nom complet + logo (CoinCap CDN)
// Usage : cryptoName('BTC') → 'Bitcoin' | cryptoLogoUrl('BTC') → URL du logo

const CRYPTO_NAMES: Record<string, string> = {
  BTC: 'Bitcoin',         ETH: 'Ethereum',        SOL: 'Solana',
  BNB: 'BNB',             XRP: 'XRP',              ADA: 'Cardano',
  DOGE: 'Dogecoin',       AVAX: 'Avalanche',       LINK: 'Chainlink',
  DOT: 'Polkadot',        MATIC: 'Polygon',        POL: 'Polygon',
  UNI: 'Uniswap',         LTC: 'Litecoin',         ATOM: 'Cosmos',
  TRX: 'TRON',            ICP: 'Internet Computer', FIL: 'Filecoin',
  NEAR: 'NEAR Protocol',  ARB: 'Arbitrum',          OP: 'Optimism',
  INJ: 'Injective',       AAVE: 'Aave',             CRV: 'Curve',
  MKR: 'Maker',           SNX: 'Synthetix',         COMP: 'Compound',
  SUSHI: 'SushiSwap',     YFI: 'Yearn Finance',     ENJ: 'Enjin Coin',
  MANA: 'Decentraland',   SAND: 'The Sandbox',      AXS: 'Axie Infinity',
  GALA: 'Gala',           CHZ: 'Chiliz',            SUI: 'Sui',
  SEI: 'Sei',             PEPE: 'Pepe',             WIF: 'dogwifhat',
  BONK: 'Bonk',           FLOKI: 'Floki',           SHIB: 'Shiba Inu',
  FRONT: 'Frontier',      OCEAN: 'Ocean Protocol',  FET: 'Fetch.ai',
  RNDR: 'Render',         GRT: 'The Graph',         ROSE: 'Oasis Network',
  ALGO: 'Algorand',       VET: 'VeChain',           HBAR: 'Hedera',
  EGLD: 'MultiversX',     THETA: 'Theta Network',   FTM: 'Fantom',
  FLOW: 'Flow',           KSM: 'Kusama',            ZEC: 'Zcash',
  XMR: 'Monero',          BCH: 'Bitcoin Cash',      ETC: 'Ethereum Classic',
  DASH: 'Dash',           ZIL: 'Zilliqa',           BLUR: 'Blur',
  GMX: 'GMX',             LDO: 'Lido DAO',          RUNE: 'THORChain',
  KAVA: 'Kava',           QNT: 'Quant',             TIA: 'Celestia',
  JUP: 'Jupiter',         PYTH: 'Pyth Network',     STRK: 'Starknet',
  NOT: 'Notcoin',         EIGEN: 'Eigenlayer',       PNUT: 'Peanut the Squirrel',
  GOAT: 'GOAT',           USUAL: 'Usual',            MOVE: 'Movement',
  PENGU: 'Pudgy Penguins', VIRTUAL: 'Virtuals Protocol', TRUMP: 'TRUMP',
  KAITO: 'Kaito',         BERA: 'Berachain',         HYPE: 'Hyperliquid',
  ONDO: 'Ondo',           ENA: 'Ethena',             WLD: 'Worldcoin',
  TON: 'Toncoin',         APT: 'Aptos',              DYDX: 'dYdX',
  IMX: 'Immutable',       MINA: 'Mina Protocol',     XLM: 'Stellar',
  JASMY: 'JasmyCoin',     MASK: 'Mask Network',      STX: 'Stacks',
  RPL: 'Rocket Pool',     CFX: 'Conflux',            MAGIC: 'MAGIC',
  W: 'Wormhole',          DYM: 'Dymension',          AGIX: 'SingularityNET',
  HIGH: 'Highstreet',     ONE: 'Harmony',            ANKR: 'Ankr',
  BAND: 'Band Protocol',  AUDIO: 'Audius',            CTSI: 'Cartesi',
  RAD: 'Radicle',         REN: 'Ren',                OMG: 'OMG Network',
  PAXG: 'PAX Gold',       TRB: 'Tellor',             GLMR: 'Moonbeam',
}

export function cryptoName(ticker: string): string {
  return CRYPTO_NAMES[ticker.toUpperCase()] ?? ticker
}

/** Logo via CoinCap CDN — fallback géré côté template avec @error */
export function cryptoLogoUrl(ticker: string): string {
  return `https://assets.coincap.io/assets/icons/${ticker.toLowerCase()}@2x.png`
}
