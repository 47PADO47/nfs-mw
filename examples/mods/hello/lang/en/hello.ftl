### The hello mod's own messages (namespace mod:example.hello).

hello-greeting = { $events ->
    [one] Hello! One event so far.
   *[other] Hello! { $events } events so far.
}
