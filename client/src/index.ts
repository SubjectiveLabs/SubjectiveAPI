export type Route = {
    full_name: string,
    agency: string,
    name: string,
    id: string,
}

export type Stop = {
    id: string,
    name: string,
    latitude: number,
    longitude: number,
};

/**
 * TypeScript client for the Subjective API.
 *
 * @param {string} baseUrl The base URL of the API.
 */
export class Client {
    /**
     * The base URL of the API.
     */
    baseUrl: string
    constructor(baseUrl: string) {
        this.baseUrl = baseUrl;
    }
    /**
     * Return routes that match a given route name such as `601`.
     * @param {string} routeName Route name to search for. It must match exactly.
     * @returns {Promise<Route[]>} A list of routes that match the search query.
     */
    async listRoutes(routeName: string): Promise<Route[]> {
        return await (
            await fetch(`${this.baseUrl}/transport/routes?query=${routeName}`)
        ).json() as Route[];
    }
    /**
     * Return stops for a given route ID and agency ID.
     * @param {string} routeId Route ID to find stops for.
     * @param {string} agencyId ID of the agency with the route.
     * @returns {Promise<Stop[]>} A list of stops for the given route.
     */
    async listStopsForRoute(routeId: string, agencyId: string): Promise<Stop[]> {
        return await (
            await fetch(`${this.baseUrl}/transport/stops?agency=${agencyId}&id=${routeId}`)
        ).json() as Stop[];
    }
}
