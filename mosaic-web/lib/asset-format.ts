function formatWithCommas(value: bigint): string {
  const raw = value.toString()
  return raw.replace(/\B(?=(\d{3})+(?!\d))/g, ',')
}

export function formatAssetSupply(maxSupply: string, decimals: number): string {
  if (maxSupply === '0') {
    return '<Unknown>'
  }

  try {
    const raw = BigInt(maxSupply)

    if (decimals === 0) {
      return formatWithCommas(raw)
    }

    const scale = BigInt(10) ** BigInt(decimals)
    const whole = raw / scale
    const fraction = raw % scale

    const wholePart = formatWithCommas(whole)

    if (fraction === 0n) {
      return wholePart
    }

    const fractionalStr = fraction
      .toString()
      .padStart(decimals, '0')
      .replace(/0+$/, '')

    if (fractionalStr.length === 0) {
      return wholePart
    }

    return `${wholePart}.${fractionalStr}`
  } catch (error) {
    console.warn('Failed to format asset supply', error)
    return '<Unknown>'
  }
}
