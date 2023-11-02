export function asset_path(relative_path) {
    let assets_path = document.body.getAttribute('data-assets-path');
    return (assets_path ? assets_path : '') + '/' + relative_path;
}
