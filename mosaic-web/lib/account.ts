/* eslint-disable @typescript-eslint/no-explicit-any */
// This file uses 'any' for Miden SDK types because they must be dynamically imported
// and cannot be statically typed at build time due to WebAssembly constraints

import { bech32m } from 'bech32'
import { type Word, type Account } from '@demox-labs/miden-sdk'

export const accountIdFromBehc32 = (addr: string): { hex: string, prefix: string } => {
  const { words, prefix } = bech32m.decode(addr)
  const full = Uint8Array.from(bech32m.fromWords(words))
  const noLead = (full.length > 0 && full[0] === 0x00) ? full.slice(1) : full
  const accountId = noLead.slice(0, 15)
  return { hex: Buffer.from(accountId).toString('hex'), prefix }
}

// Helper function that accepts SDK types as parameters
// This allows the page to dynamically import the SDK and pass it here
export const getOrImportAccount = async (
  client: any, // WebClient type
  AccountId: any, // AccountId constructor
  accountBech32: string
) => {
  const accountIdRaw = accountIdFromBehc32(accountBech32)
  const accountHex = '0x' + accountIdRaw.hex
  console.log('Account hex:', accountHex)

  const accountId = AccountId.fromHex(accountHex)
  let account = await client.getAccount(accountId)

  if (!account) {
    account = await client.importAccountById(accountId)
  }

  return account
}

// Helper to read account storage
export const readAccountStorage = (
  account: Account, // Account type
  Felt: any, // Felt constructor
  Word: any // Word constructor
) => {
  const head = account.storage().getItem(2)?.toFelts()
  let mapValue = null

  if (head) {
    mapValue = account.storage().getMapItem(
      1,
      Word.newFromFelts([new Felt(BigInt(1)), new Felt(BigInt(0)), new Felt(BigInt(0)), new Felt(BigInt(0))])
    )
  }

  return {
    head: head?.map((x: any) => x.toString()),
    mapValue: mapValue?.toFelts().map((x: any) => x.toString()),
  }
}

type Pair = { base: { symbol: string, faucet: string }, quote: { symbol: string, faucet: string } }

type Quote = { amount: bigint, price: bigint }

type DeskInfo = {
  pair: Pair,
  quotes: {
    sell: Quote[],
    buy: Quote[]
  }
}

export const getDeskInfo = (Word: any, Felt: any, account: Account | null | undefined): DeskInfo | null => {
  if (!account) return null

  try {
    const base = account.storage().getItem(1)
    const quote = account.storage().getItem(2)
    if(!base || !quote) return null
    const pair = decodePair(base, quote)
    const sell = getSQuotes(Word, Felt, account, 7, 6)
    const buy = getSQuotes(Word, Felt, account, 12, 11)
    if(!sell || !buy) return null
    return { pair, quotes: { sell, buy } }
  } catch (error) {
    console.error('Error getting desk info:', error)
    return null
  }
}


const getSQuotes = (Word: any, Felt: any, account: Account, startSlot: number, bookSlot: number): { amount: bigint, price: bigint }[] | null => {
  const quotes = []
  let id: bigint | undefined = account.storage().getItem(startSlot)?.toU64s()[0]
  if(!id) return null

  while(id !== 0n) {
    const z0 = new Felt(BigInt(0))
    const z1 = new Felt(BigInt(0))
    const z2 = new Felt(BigInt(0))
    const entry = account.storage().getMapItem(bookSlot, Word.newFromFelts([new Felt(id), z0, z1, z2]))
    if(entry) {
      const [, next, price, amount] = entry?.toU64s().reverse()
      quotes.push({ price, amount })
      id = next
    } else {
      console.log('Algorithm error: entry not found')
      return null
    }
  }
  return quotes
}

const decodePair = (base: Word, quote: Word): { base: { symbol: string, faucet: string }, quote: { symbol: string, faucet: string } } => {
  const decodeSymbol = (word: Word): { symbol: string, faucet: string } => {
    // Convert Word to array of Felts
    const felts = word.toFelts()

    // felts[0] contains the symbol packed as big-endian ASCII (up to 8 chars)
    // felts[2] contains upper 7 bytes of faucet (with leading zero)
    // felts[3] contains lower 8 bytes of faucet

    // Extract symbol from felts[0]
    const symbolBigInt = BigInt(felts[0].toString())
    const symbolBytes: number[] = []

    // Unpack 8 bytes in big-endian order
    for (let i = 7; i >= 0; i--) {
      const byte = Number((symbolBigInt >> BigInt(i * 8)) & BigInt(0xFF))
      if (byte !== 0) { // Skip trailing zeros
        symbolBytes.push(byte)
      }
    }

    const symbol = String.fromCharCode(...symbolBytes)

    // Extract faucet AccountId from felts[2] and felts[3]
    const hiBigInt = BigInt(felts[2].toString())
    const loBigInt = BigInt(felts[3].toString())

    // felts[2] has format: [0, b0, b1, b2, b3, b4, b5, b6] (7 bytes after leading zero)
    // felts[3] has format: [b7, b8, b9, b10, b11, b12, b13, b14] (8 bytes)
    const faucetBytes = new Uint8Array(15)

    // Extract 7 bytes from hi (skip first byte which is 0)
    for (let i = 0; i < 7; i++) {
      faucetBytes[i] = Number((hiBigInt >> BigInt((6 - i) * 8)) & BigInt(0xFF))
    }

    // Extract 8 bytes from lo
    for (let i = 0; i < 8; i++) {
      faucetBytes[7 + i] = Number((loBigInt >> BigInt((7 - i) * 8)) & BigInt(0xFF))
    }

    // Convert to bech32m format
    const words = bech32m.toWords(faucetBytes)
    const faucet = bech32m.encode('mtst', words)

    return { symbol, faucet }
  }

  return {
    base: decodeSymbol(base),
    quote: decodeSymbol(quote),
  }
}