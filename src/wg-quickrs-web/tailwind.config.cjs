/** @type {import('tailwindcss').Config} */
module.exports = {
    content: [
        "./index.html",
        "./src/**/*.{vue, js, ts, jsx, ts×}",
    ],
    darkMode: 'class',
    theme: {
        extend: {
            colors: {
                // Semantic color tokens using CSS variables
                'page': 'var(--bg-page)',
                'card': 'var(--bg-card)',
                'header': 'var(--bg-header)',
                'button': 'var(--bg-button)',
                'button-hover': 'var(--bg-button-hover)',
                'input': 'var(--bg-input)',
                'dropdown': 'var(--bg-dropdown)',
                'backdrop': 'var(--bg-backdrop)',
                'text-primary': 'var(--text-primary)',
                'text-secondary': 'var(--text-secondary)',
                'text-muted': 'var(--text-muted)',
                'text-icon': 'var(--text-icon)',
                'text-button': 'var(--text-button)',
                'border-default': 'var(--border-default)',
                'border-divider': 'var(--border-divider)',
                'border-input': 'var(--border-input)',
                'border-input-error': 'var(--border-input-error)',
                'badge-success': {
                    'bg': 'var(--badge-success-bg)',
                    'text': 'var(--badge-success-text)',
                },
                'badge-error': {
                    'bg': 'var(--badge-error-bg)',
                    'text': 'var(--badge-error-text)',
                },
                'badge-warning': {
                    'bg': 'var(--badge-warning-bg)',
                    'text': 'var(--badge-warning-text)',
                },
                'badge-info': {
                    'bg': 'var(--badge-info-bg)',
                    'text': 'var(--badge-info-text)',
                },
            },
        },
    },
    plugins: [],
}
