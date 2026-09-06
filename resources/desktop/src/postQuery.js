// Calendar needs every post in its visible date range; library pagination stays explicit.
export function paginateItems(items, requestedPage, perPage = 25) {
    const size = Math.max(1, Math.trunc(perPage) || 25);
    const pages = Math.max(1, Math.ceil(items.length / size));
    const page = Math.max(1, Math.min(pages, Math.trunc(requestedPage) || 1));
    const offset = (page - 1) * size;
    return {
        items: items.slice(offset, offset + size),
        page,
        pages,
        total: items.length,
        start: items.length ? offset + 1 : 0,
        end: Math.min(offset + size, items.length),
    };
}

export async function queryPostPages(query, request, { allPages = false, isCurrent = () => true } = {}) {
    const result = await query(request);
    if (!isCurrent()) return null;
    if (allPages) {
        for (let page = 2; page <= result.total_pages; page += 1) {
            const next = await query({ ...request, page });
            if (!isCurrent()) return null;
            result.items.push(...next.items);
        }
    }
    return result;
}
