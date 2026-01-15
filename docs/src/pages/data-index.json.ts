import type { APIRoute } from "astro";
import Fuse from "fuse.js";
import { getDocsMetadata } from "./data.json";
import { FuseConfig } from "./fuse.config.ts";

/**
 * Dynamically generate a json with all the metadata from the certifications
 * @param param0 Astro API route
 */
export const GET: APIRoute = async () => {
    const index = Fuse.createIndex(
        FuseConfig.keys!,
        await getDocsMetadata(),
    );

    return Response.json(index.toJSON());
};
