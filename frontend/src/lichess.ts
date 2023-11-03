import { OAuth2AuthCodePKCE, HttpClient } from '@bity/oauth2-auth-code-pkce';

const lichess_host = 'https://lichess.org';
const scopes = ['puzzle:read'];
const client_id = 'better-tactics';
const client_url = (() => {
    const url = new URL(location.href);
    url.search = '';
    return url.href;
})();

export class LichessApiClient {
    private fetch_client: HttpClient = null;

    private oauth = new OAuth2AuthCodePKCE({
        authorizationUrl: `${lichess_host}/oauth`,
        clientId: client_id,
        onAccessTokenExpiry: refreshAccessToken => refreshAccessToken(),
        onInvalidGrant: _retry => {},
        redirectUrl: client_url,
        scopes,
        tokenUrl: `${lichess_host}/api/token`,
    });

    // Initialise the client by authorizing with the lichess api if necessary.
    async init() {
        console.log('Initialising lichess api client...');

        let authorized = this.oauth.isAuthorized();
        if (authorized) {
            console.log('Already authorized');
            return;
        }

        let returning;
        try {
            returning = await this.oauth.isReturningFromAuthServer();
            console.log('returning from auth server: ' + returning);
        }
        catch (e) {
            throw new Error(`Error when checking if returning from auth server: ${e}`);
        }

        // If we aren't authorized and aren't returning from the auth server right now, redirect
        // there.
        if (!returning) {
            try {
                await this.oauth.fetchAuthorizationCode();
                return;
            }
            catch (e) {
                throw new Error(`Error redirecting to auth server: ${e}`);
            }
        }

        // Now we should be returning from the auth server and should just be able to call
        // getAccessToken to get a new access token.
        try {
            await this.oauth.getAccessToken();
            console.log('Got access token');
        }
        catch (e) {
            throw new Error(`Error requesting access token: ${e}`);
        }
    }

    logout() {
        console.log("Logging out");
        this.oauth.reset();
    }

    // Wrap fetch client and add lichess base url.
    fetch(endpoint: string, options: any = {}) {
        if (this.fetch_client == null) {
            // Decorate fetch client so we can make api requests without manually specifying
            // authorization headers.
            console.log("Initialising fetch client");
            this.fetch_client = this.oauth.decorateFetchHTTPClient(window.fetch);
        }

        let url = lichess_host + (endpoint[0] != '/' ? '/' : '') + endpoint;
        return this.fetch_client(url, options);
    }
}
