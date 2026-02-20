// HTMX SSE Extension - Minimal implementation for mkube-console
// Connects to Server-Sent Events endpoints and swaps content on events
(function() {
    if (typeof htmx === 'undefined') return;

    htmx.defineExtension('sse', {
        onEvent: function(name, evt) {
            if (name === 'htmx:beforeCleanupElement') {
                var elt = evt.target || evt.detail.elt;
                if (elt && elt._sseEventSource) {
                    elt._sseEventSource.close();
                    delete elt._sseEventSource;
                }
            }
        },

        init: function(api) {
            // Process sse-connect attributes
            document.addEventListener('htmx:afterProcessNode', function(evt) {
                var elt = evt.detail.elt;
                if (!elt.getAttribute) return;

                var sseConnect = elt.getAttribute('sse-connect');
                if (!sseConnect) return;

                var eventSource = new EventSource(sseConnect);
                elt._sseEventSource = eventSource;

                // Handle sse-swap elements (children that want specific events)
                var swapElts = elt.querySelectorAll('[sse-swap]');
                swapElts.forEach(function(swapElt) {
                    var eventName = swapElt.getAttribute('sse-swap');
                    eventSource.addEventListener(eventName, function(e) {
                        htmx.swap(swapElt, e.data, {swapStyle: 'innerHTML'});
                    });
                });

                // Also listen on the element itself
                var selfSwap = elt.getAttribute('sse-swap');
                if (selfSwap) {
                    eventSource.addEventListener(selfSwap, function(e) {
                        htmx.swap(elt, e.data, {swapStyle: 'innerHTML'});
                    });
                }

                eventSource.onerror = function() {
                    // Reconnect handled automatically by EventSource
                };
            });
        }
    });
})();
