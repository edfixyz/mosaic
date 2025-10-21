import { auth0 } from '@/lib/auth0';

export async function GET() {
  try {
    const session = await auth0.getSession();

    if (!session) {
      return Response.json(
        { error: 'Not authenticated' },
        { status: 401 }
      );
    }

    const tokenResult = await auth0.getAccessToken({
      audience: process.env.AUTH0_AUDIENCE || 'https://api.mosaic.edfi.xyz',
      scope: 'openid profile email',
    });

    if (!tokenResult || !tokenResult.token) {
      return Response.json(
        { error: 'Failed to get access token' },
        { status: 500 }
      );
    }

    return Response.json({ accessToken: tokenResult.token });
  } catch (error) {
    console.error('Error getting access token:', error);
    return Response.json(
      { error: 'Failed to get access token', details: String(error) },
      { status: 500 }
    );
  }
}
