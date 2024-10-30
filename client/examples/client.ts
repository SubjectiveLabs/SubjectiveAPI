import { Client } from "subjective-api";

const client = new Client("https://api.subjective.school/v1");
const routes = (await client.listRoutes("601")).slice(0, 2);
const route = routes[0];
const stops = (await client.listStopsForRoute(route.id, route.agency)).slice(0, 2);
console.table({ routes, stops });
