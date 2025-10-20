import { auth0 } from '@/lib/auth0';

export async function GET() {
  try {
    const session = await auth0.getSession();

    if (!session) {
      return Response.json(null, { status: 401 });
    }

    return Response.json({
      email: session.user.email,
      name: session.user.name,
      picture: session.user.picture,
      sub: session.user.sub,
    });
  } catch (error) {
    console.error('Error getting user session:', error);
    return Response.json(null, { status: 500 });
  }
}
