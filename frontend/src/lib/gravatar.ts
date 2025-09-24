import CryptoJS from 'crypto-js';

/**
 * Generate a Gravatar URL for an email address
 * @param email - The email address to generate the Gravatar URL for
 * @param size - The size of the avatar (default: 80)
 * @param defaultImage - Default image type if no Gravatar exists (default: 'identicon')
 * @returns The Gravatar URL
 */
export function getGravatarUrl(
	email: string,
	size: number = 80,
	defaultImage: string = 'identicon'
): string {
	if (!email) {
		return `https://www.gravatar.com/avatar/?s=${size}&d=${defaultImage}`;
	}

	// Normalize email: trim whitespace and convert to lowercase
	const normalizedEmail = email.trim().toLowerCase();

	// Create MD5 hash of the email (required by Gravatar)
	const hash = CryptoJS.MD5(normalizedEmail).toString();

	// Build Gravatar URL
	return `https://www.gravatar.com/avatar/${hash}?s=${size}&d=${defaultImage}`;
}

/**
 * Get user initials from name or email
 * @param name - The user's display name
 * @param email - The user's email address
 * @returns User initials (1-2 characters)
 */
export function getUserInitials(name?: string, email?: string): string {
	if (name && name.trim()) {
		const parts = name.trim().split(/\s+/);
		if (parts.length >= 2) {
			// First letter of first and last name
			return (parts[0].charAt(0) + parts[parts.length - 1].charAt(0)).toUpperCase();
		} else {
			// First letter of name
			return parts[0].charAt(0).toUpperCase();
		}
	}

	if (email && email.trim()) {
		// First letter of email username
		const username = email.split('@')[0];
		return username.charAt(0).toUpperCase();
	}

	return 'U'; // Ultimate fallback
}
