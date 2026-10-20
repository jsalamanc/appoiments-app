pub const CATALOG_PAGE_SIZE: i64 = 9;

pub fn get_total_pages(total: i64, page_size: i64) -> i64 {
    let page_size = if page_size <= 0 { CATALOG_PAGE_SIZE } else { page_size };
    std::cmp::max(1, (total as f64 / page_size as f64).ceil() as i64)
}

/// Devuelve (limit, offset) para una página 1-indexada.
pub fn get_range(page: i64, page_size: i64) -> (i64, i64) {
    let page_size = if page_size <= 0 { CATALOG_PAGE_SIZE } else { page_size };
    let page = std::cmp::max(1, page);
    let offset = (page - 1) * page_size;
    (page_size, offset)
}
