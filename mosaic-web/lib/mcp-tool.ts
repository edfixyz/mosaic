import { callMCPTool, RawCallToolResult } from '@/lib/mcp-client'

export type NetworkName = 'Testnet' | 'Localnet'

type EmptyArgs = Record<string, never>

export type MarketCurrency = {
  code: string
  issuer: string
}

export type MarketDescription = {
  base: MarketCurrency
  quote: MarketCurrency
}

export type CreateClientAccountResponse = {
  success: boolean
  account_id: string
  name?: string
}

export type CreateDeskAccountResponse = {
  success: boolean
  account_id: string
  desk_uuid: string
  market: MarketDescription
}

export type CreateLiquidityAccountResponse = {
  success: boolean
  account_id: string
}

export type CreateFaucetAccountResponse = {
  success: boolean
  account_id: string
  token_symbol: string
  decimals: number
  max_supply: number
}

export type AccountInfo = {
  account_id: string
  network: string
  account_type: string
  name?: string | null
}

export type ListAccountsResponse = {
  success: boolean
  accounts: AccountInfo[]
}

export type ClientSyncResponse = {
  success: boolean
  block_num: number
  new_public_notes: number
  committed_notes: number
  consumed_notes: number
  updated_accounts: number
}

export type CreateOrderResponse = {
  success: boolean
  note: unknown
}

export type CreateRawNoteResponse = {
  success: boolean
  note: unknown
}

export type GetAccountStatusResponse = {
  success: boolean
  account_id: string
  storage_mode: string
  account_type: string
  assets: Array<{
    faucet: string
    amount: number
    fungible: boolean
  }>
}

export type ConsumeNoteResponse = {
  success: boolean
  transaction_id: string
}

export type DeskPushNoteResponse = {
  success: boolean
  desk_uuid: string
  note_id: number
}

export type GetDeskInfoResponse = {
  success: boolean
  desk_uuid: string
  account_id: string
  network: string
  market: MarketDescription
}

export type FlushResponse = {
  success: boolean
  clients_flushed: number
}

export type VersionResponse = {
  success: boolean
  version: string
}

export type AssetSummary = {
  account: string
  symbol: string
  maxSupply: string
  decimals: number
  verified: boolean
  owner: boolean
  hidden: boolean
}

export type ListAssetsResponse = AssetSummary[]

export type RegisterAssetResponse = {
  success: boolean
}

export type ToolDefinitions = {
  create_client_account: {
    args: { network: NetworkName; name?: string }
    result: CreateClientAccountResponse
  }
  create_desk_account: {
    args: { network: NetworkName; market: MarketDescription }
    result: CreateDeskAccountResponse
  }
  create_liquidity_account: {
    args: { network: NetworkName }
    result: CreateLiquidityAccountResponse
  }
  create_faucet_account: {
    args: {
      token_symbol: string
      decimals: number
      max_supply: number
      network: NetworkName
    }
    result: CreateFaucetAccountResponse
  }
  list_accounts: {
    args: EmptyArgs
    result: ListAccountsResponse
  }
  client_sync: {
    args: { network: NetworkName }
    result: ClientSyncResponse
  }
  create_order: {
    args: {
      network: NetworkName
      account_id: string
      order: unknown
      commit?: boolean
    }
    result: CreateOrderResponse
  }
  create_raw_note: {
    args: {
      network: NetworkName
      account_id: string
      note_type: string
      program: string
      libraries?: Array<[string, string]>
      inputs?: Array<[string, unknown]>
      note_secret?: [number, number, number, number]
    }
    result: CreateRawNoteResponse
  }
  get_account_status: {
    args: {
      network: NetworkName
      account_id: string
    }
    result: GetAccountStatusResponse
  }
  consume_note: {
    args: {
      network: NetworkName
      account_id: string
      miden_note: unknown
    }
    result: ConsumeNoteResponse
  }
  desk_push_note: {
    args: {
      desk_uuid: string
      note: unknown
    }
    result: DeskPushNoteResponse
  }
  get_desk_info: {
    args: {
      desk_uuid: string
    }
    result: GetDeskInfoResponse
  }
  flush: {
    args: EmptyArgs
    result: FlushResponse
  }
  version: {
    args: EmptyArgs
    result: VersionResponse
  }
  list_assets: {
    args: EmptyArgs
    result: ListAssetsResponse
  }
  register_asset: {
    args: {
      symbol: string
      account: string
      max_supply: string
      decimals: number
      verified?: boolean
      owner?: boolean
      hidden?: boolean
    }
    result: RegisterAssetResponse
  }
}

export type ToolName = keyof ToolDefinitions
export type ToolArgs<Name extends ToolName> = ToolDefinitions[Name]['args']
export type ToolResult<Name extends ToolName> = ToolDefinitions[Name]['result']

function parseToolResult<T>(tool: string, raw: RawCallToolResult): T {
  if (raw.isError) {
    throw new Error(`MCP tool '${tool}' responded with an error`)
  }

  if (raw.structuredContent !== undefined) {
    return raw.structuredContent as T
  }

  if (raw.content && raw.content.length > 0) {
    const textPayload = raw.content
      .map((item) => item.text ?? '')
      .join('')
      .trim()

    if (textPayload.length > 0) {
      try {
        return JSON.parse(textPayload) as T
      } catch (error) {
        throw new Error(
          `Failed to parse MCP tool '${tool}' response as JSON: ${String(error)}`
        )
      }
    }
  }

  throw new Error(`MCP tool '${tool}' returned no parsable content`)
}

function normalizeArgs(args: Record<string, unknown> | EmptyArgs): Record<string, unknown> {
  if (!args) {
    return {}
  }
  return args
}

export async function callMcpTool<Name extends ToolName>(
  name: Name,
  args: ToolArgs<Name>,
  accessToken?: string | null
): Promise<ToolResult<Name>> {
  const raw = await callMCPTool(name, normalizeArgs(args as Record<string, unknown>), accessToken)
  return parseToolResult<ToolResult<Name>>(name, raw)
}
