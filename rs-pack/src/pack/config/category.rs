use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use tracing::debug;

pub fn pack_categories(registry: &PackRegistry) -> anyhow::Result<PackedFile> {
    let category = &registry.category;

    debug!("  Found {} .categories", category.max);

    let mut server = PackedData::new(category.max);

    for id in 0..category.max {
        server.start_entry();

        let Some(debugname) = category.get_by_id(id) else {
            panic!("Unknown category id: {}", id);
        };

        // 1
        server.p1(1);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: None,
    })
}
