import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js'

type ManagedClient = {
  client: Client
  transport: StreamableHTTPClientTransport
}

export interface RawToolContent {
  type: string
  text?: string
}

export interface RawCallToolResult {
  content?: RawToolContent[]
  structuredContent?: unknown
  isError?: boolean
  error?: unknown
}

const MCP_SERVER_URL =
  process.env.NEXT_PUBLIC_MCP_SERVER_URL ?? 'http://localhost:8000/mcp'

const clientCache = new Map<string, ManagedClient>()

function createTransport(accessToken: string) {
  return new StreamableHTTPClientTransport(new URL(MCP_SERVER_URL), {
    requestInit: {
      headers: {
        Authorization: `Bearer ${accessToken}`,
        Accept: 'application/json, text/event-stream',
      },
    },
  })
}

async function getClient(accessToken: string): Promise<Client> {
  const cached = clientCache.get(accessToken)
  if (cached) {
    return cached.client
  }

  const transport = createTransport(accessToken)
  const client = new Client(
    { name: 'mosaic-web', version: '1.0.0' },
    {
      capabilities: {
        tools: {},
        resources: {},
        prompts: {},
      },
    }
  )

  try {
    await client.connect(transport)
  } catch (error) {
    await transport.close().catch(() => undefined)
    throw error
  }

  client.onclose = () => {
    clientCache.delete(accessToken)
  }

  clientCache.set(accessToken, { client, transport })
  return client
}

export async function callMCPTool(
  toolName: string,
  args: Record<string, unknown> = {},
  accessToken: string
): Promise<RawCallToolResult> {
  const client = await getClient(accessToken)
  const result = await client.callTool({ name: toolName, arguments: args })
  return result as RawCallToolResult
}

export async function ensureMCPConnection(accessToken: string): Promise<void> {
  await getClient(accessToken)
}

export async function listMCPTools(
  accessToken: string
): Promise<unknown> {
  const client = await getClient(accessToken)
  return client.listTools({})
}

export async function resetMCPSession(): Promise<void> {
  const entries = Array.from(clientCache.values())
  clientCache.clear()

  await Promise.all(
    entries.map(async ({ client, transport }) => {
      try {
        await client.close()
      } catch (error) {
        console.warn('Failed to close MCP client', error)
      }

      try {
        await transport.close()
      } catch (error) {
        console.warn('Failed to close MCP transport', error)
      }
    })
  )
}
