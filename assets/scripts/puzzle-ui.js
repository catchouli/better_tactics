import {
    init,
    classModule,
    propsModule,
    styleModule,
    eventListenersModule,
    h,
} from '../deps/snabbdom.js';

const patch = init([
  // Init patch function with chosen modules
  classModule, // makes it easy to toggle classes
  propsModule, // for setting properties on DOM elements
  styleModule, // handles styling on elements with support for animations
  eventListenersModule, // attaches event listeners
]);

function view(currentDate) {
    return h('div', 'Current date ' + currentDate);
}

let vnode = patch($("#topper-bar")[0], view(new Date()));

setInterval(function() {
    const newVNode = view(new Date());
    vnode = patch(vnode, newVNode);
}, 1000);
