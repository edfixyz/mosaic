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

export type ClientAccountInfo = {
  account_id: string
  network: string
  account_type: string
  name?: string | null
}

export type DeskAccountInfo = {
  account_id: string
  network: string
  market: MarketDescription
  owner_account: string
  market_url: string
}

export type ListAccountsResponse = {
  success: boolean
  client_accounts: ClientAccountInfo[]
  desk_accounts: DeskAccountInfo[]
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
  desk_account: string
  note_id: number
}

export type GetDeskInfoResponse = {
  success: boolean
  desk_account: string
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

export type StoredOrderSummary = {
  uuid: string
  order_type: string
  order_json: string
  stage: string
  status: string
  account: string
  created_at?: string | null
}

export type RoleSettings = {
  is_client: boolean
  is_liquidity_provider: boolean
  is_desk: boolean
}

export type OrderSide = 'BUY' | 'SELL'

type OrderUuid = string | number
type OrderAmount = number
type OrderPrice = number

type QuoteRequestOrder = {
  QuoteRequest: {
    market: string
    uuid: OrderUuid
    side: OrderSide
    amount: OrderAmount
  }
}

type QuoteRequestOfferOrder = {
  QuoteRequestOffer: {
    market: string
    uuid: OrderUuid
    side: OrderSide
    amount: OrderAmount
    price: OrderPrice
  }
}

type QuoteRequestNoOfferOrder = {
  QuoteRequestNoOffer: {
    market: string
    uuid: OrderUuid
  }
}

type LimitOrder = {
  LimitOrder: {
    market: string
    uuid: OrderUuid
    side: OrderSide
    amount: OrderAmount
    price: OrderPrice
  }
}

type LiquidityOfferOrder = {
  LiquidityOffer: {
    market: string
    uuid: OrderUuid
    side: OrderSide
    amount: OrderAmount
    price: OrderPrice
  }
}

type FundAccountOrder = {
  FundAccount: {
    target_account_id: string
    amount: OrderAmount
  }
}

type KycpassedOrder = {
  KYCPassed: {
    market: string
  }
}

type OrderUnitVariant =
  | 'LimitBuyOrderLocked'
  | 'LimitBuyOrderNotLocked'
  | 'LimitSellOrderLocked'
  | 'LimitSellOrderNotLocked'

export type OrderPayload =
  | KycpassedOrder
  | QuoteRequestOfferOrder
  | QuoteRequestNoOfferOrder
  | QuoteRequestOrder
  | LimitOrder
  | LiquidityOfferOrder
  | FundAccountOrder
  | OrderUnitVariant

type AccountOrderCreateClient = {
  CreateClient: {
    network: NetworkName
    name?: string | null
  }
}

type AccountOrderCreateDesk = {
  CreateDesk: {
    network: NetworkName
    market: MarketDescription
    owner_account: string
  }
}

type AccountOrderCreateFaucet = {
  CreateFaucet: {
    network: NetworkName
    token_symbol: string
    decimals: number
    max_supply: number
  }
}

type AccountOrderCreateLiquidity = {
  CreateLiquidity: {
    network: NetworkName
  }
}

type AccountOrderActivateDesk = {
  ActivateDesk: {
    desk_account: string
    owner_account: string
  }
}

type AccountOrderDeactivateDesk = {
  DeactivateDesk: {
    desk_account: string
    owner_account: string
  }
}

export type AccountOrderPayload =
  | AccountOrderCreateClient
  | AccountOrderCreateDesk
  | AccountOrderCreateFaucet
  | AccountOrderCreateLiquidity
  | AccountOrderActivateDesk
  | AccountOrderDeactivateDesk

export type AccountOrderResultPayload =
  | {
      Client: {
        account_id: string
        name?: string | null
      }
    }
  | {
      Desk: {
        account_id: string
        market: MarketDescription
        owner_account: string
        market_url: string
      }
    }
  | {
      DeskActivated: {
        desk_account: string
        owner_account: string
      }
    }
  | {
      DeskDeactivated: {
        desk_account: string
        owner_account: string
      }
    }
  | {
      Faucet: {
        account_id: string
        token_symbol: string
        decimals: number
        max_supply: number
      }
    }
  | {
      Liquidity: {
        account_id: string
      }
    }

export type CreateAccountOrderResponse = {
  success: boolean
  result: AccountOrderResultPayload
}

export type ToolDefinitions = {
  create_account_order: {
    args: { order: AccountOrderPayload }
    result: CreateAccountOrderResponse
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
      order: OrderPayload
      commit?: boolean
    }
    result: CreateOrderResponse
  }
  list_orders: {
    args: EmptyArgs
    result: StoredOrderSummary[]
  }
  get_role_settings: {
    args: EmptyArgs
    result: RoleSettings
  }
  update_role_settings: {
    args: RoleSettings
    result: RoleSettings
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
      desk_account: string
      note: unknown
    }
    result: DeskPushNoteResponse
  }
  get_desk_info: {
    args: {
      desk_account: string
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
