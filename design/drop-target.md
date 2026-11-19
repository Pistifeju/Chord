# Drop target

While the user drags an image over a chat pane that takes attachments, a card with a 2px dashed `accent-ink` outline sits 8px inside the pane: `accent-wash` over `sheet` at 85%, so the transcript shows faintly through. In its center sit a 48px `accent` circle with a 24px image icon in `on-accent`, the title "Drop images to attach" (15px, 600) and the line "They go with your next message in this chat." in `ink-muted`. The card fades in over 200ms; its content fades in and rises 4px with it, using `--ease-out`. It closes at once. The window's folder drop stays off for images.

While the user drags a folder over the window, a 2px dashed `accent-ink` outline sits 12px inside the window over the scrim, with its label on a small `sheet` chip. Over the terminal the folder drop stays off too, and no outline shows: files dropped there paste their paths into the shell.
