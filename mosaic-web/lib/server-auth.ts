import { auth0 } from '@/lib/auth0'

const DEFAULT_AUDIENCE = 'https://api.mosaic.edfi.xyz'
const DEFAULT_SCOPE = 'openid profile email'

export async function getServerAccessToken(): Promise<string | null> {
  try {
    const session = await auth0.getSession()
    if (!session) {
      return null
    }

    const tokenResult = await auth0.getAccessToken({
      audience: process.env.AUTH0_AUDIENCE || DEFAULT_AUDIENCE,
      scope: DEFAULT_SCOPE,
    })

    return tokenResult?.token ?? null
  } catch (error) {
    console.error('Failed to acquire server access token:', error)
    return null
  }
}
