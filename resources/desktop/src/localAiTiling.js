import { LOCAL_AI_UPSCALE_TILE_SIZE as TILE, LOCAL_AI_UPSCALE_MAX_SOURCE_EDGE as MAX } from './localAiConfig.js';

const HALO = 16;
const STRIDE = TILE - HALO * 2;

export function localAiTiles(width, height) {
    if (![width, height].every(value => Number.isInteger(value) && value > 0 && value <= MAX)) {
        throw new Error(`4× AI upscaling supports images up to ${MAX} × ${MAX}px. Choose a smaller source; Social will not shrink your original to upscale it.`);
    }
    const tiles = [];
    // Small images use one tile. Larger images share context across boundaries;
    // discard the halo instead of stitching the model's unreliable tile edges.
    const stepX = width <= TILE ? TILE : STRIDE;
    const stepY = height <= TILE ? TILE : STRIDE;
    for (let y = 0; y < height; y += stepY) {
        for (let x = 0; x < width; x += stepX) {
            const offsetX = width <= TILE ? 0 : HALO;
            const offsetY = height <= TILE ? 0 : HALO;
            tiles.push({ x, y, width: Math.min(stepX, width - x), height: Math.min(stepY, height - y), offsetX, offsetY, inputX: x - offsetX, inputY: y - offsetY });
        }
    }
    return tiles;
}

export function localAiTileInput(pixels, width, height, tile) {
    if (pixels.length !== width * height * 4) throw new Error('Invalid source image pixels');
    const input = new Uint8Array(TILE * TILE * 3);
    for (let y = 0; y < TILE; y += 1) {
        const sourceY = Math.max(0, Math.min(height - 1, tile.inputY + y));
        for (let x = 0; x < TILE; x += 1) {
            const sourceX = Math.max(0, Math.min(width - 1, tile.inputX + x));
            const source = (sourceY * width + sourceX) * 4;
            const target = (y * TILE + x) * 3;
            input.set(pixels.subarray(source, source + 3), target);
        }
    }
    return input;
}
