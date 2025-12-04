#!/bin/bash

# Script to prepare your changes for contribution to wg-quickrs
# This script helps you create a branch, commit changes, and prepare for a PR

set -e

echo "=== wg-quickrs Contribution Preparation ==="
echo ""

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "Error: Not in a git repository"
    exit 1
fi

# Check for uncommitted changes
if git diff-index --quiet HEAD --; then
    echo "No uncommitted changes found."
    exit 0
fi

echo "Current changes:"
git status --short
echo ""

# Ask for branch name
read -p "Enter a branch name for your feature (e.g., feature/router-mode): " BRANCH_NAME
if [ -z "$BRANCH_NAME" ]; then
    echo "Error: Branch name cannot be empty"
    exit 1
fi

# Check if branch already exists
if git show-ref --verify --quiet refs/heads/"$BRANCH_NAME"; then
    read -p "Branch $BRANCH_NAME already exists. Use it? (y/n): " USE_EXISTING
    if [ "$USE_EXISTING" != "y" ]; then
        exit 1
    fi
    git checkout "$BRANCH_NAME"
else
    git checkout -b "$BRANCH_NAME"
fi

echo ""
echo "Staging all changes..."
git add .

echo ""
echo "Current changes to be committed:"
git status --short

echo ""
read -p "Enter commit message (or press Enter for default): " COMMIT_MSG

if [ -z "$COMMIT_MSG" ]; then
    COMMIT_MSG="Add router mode and firewall management features

- Add router mode functionality with PBR routing
- Implement firewall management for iptables/pf
- Add storage persistence layer
- Improve web UI with router mode dialog
- Various bug fixes and improvements"
fi

echo ""
echo "Committing changes..."
git commit -m "$COMMIT_MSG"

echo ""
echo "=== Next Steps ==="
echo ""
echo "1. Fork the repository on GitHub:"
echo "   https://github.com/GodOfKebab/wg-quickrs"
echo ""
echo "2. Add your fork as a remote:"
echo "   git remote add fork https://github.com/YOUR_USERNAME/wg-quickrs.git"
echo ""
echo "3. Push your branch:"
echo "   git push fork $BRANCH_NAME"
echo ""
echo "4. Create a Pull Request on GitHub:"
echo "   https://github.com/GodOfKebab/wg-quickrs/compare"
echo ""
echo "Your changes are now committed to branch: $BRANCH_NAME"
echo ""

