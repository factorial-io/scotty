import { writable } from 'svelte/store';

// Set the base title for the application
const BASE_TITLE = 'Scotty';

// Create a writable store for the title
const title = writable(BASE_TITLE);

// Function to set the page title
export function setTitle(pageTitle: string) {
  if (pageTitle) {
    title.set(`${pageTitle} | ${BASE_TITLE}`);
  } else {
    title.set(BASE_TITLE);
  }
}

export default title;