import type { Guide } from "./types";

const guide: Guide = {
  title: "User guide",
  subtitle: "Start from zero and learn to look after your projects with Project Hub in minutes",
  sections: [
    {
      id: "welcome",
      title: "What Project Hub does",
      summary: "Snapshots of your project folders, and a way back to any of them",
      blocks: [
        {
          type: "paragraph",
          text: "Project Hub keeps **snapshots** of your project folders. Whenever a step feels done, save; whatever breaks later, one click takes the folder back to how it was. No Git knowledge needed: everything is a button.",
        },
        { type: "heading", text: "Four words to know" },
        {
          type: "terms",
          items: [
            { term: "Project", text: "A folder on your computer: a website, a paper, a set of designs." },
            { term: "Save record", text: "A complete snapshot of the project at one moment, with a title and notes, listed by time." },
            { term: "Unsaved changes", text: "The files you edited, added or deleted since the last save." },
            { term: "Version", text: "A separate line of save records. Start one to try an idea without touching what you have." },
          ],
        },
        {
          type: "tip",
          text: "Every snapshot lives in a hidden folder inside the project, on your computer only. Uploading to GitHub is optional.",
        },
      ],
    },
    {
      id: "quickStart",
      title: "Quick start",
      summary: "Your first save in three steps",
      blocks: [
        { type: "paragraph", text: "Follow these three steps to make your first save:" },
        {
          type: "steps",
          items: [
            "**Add a project**: click **+** at the top of the Projects panel and pick a folder. Folders without version management get it switched on automatically.",
            "**Look at the changes**: in a newly added project, the existing files appear under **Unsaved changes**. From then on, edits you make in any editor show up there as they happen.",
            "**Save a version**: click **Save version…**, write a title (say, \"Home page layout done\"), add details if you like, and click **Save** (or press `Ctrl+Enter`).",
          ],
        },
        { type: "action", action: "addProject", label: "Add a project now" },
        {
          type: "tip",
          text: "The new record appears at the top of **Save history**. Save after every milestone and going back is never a gamble.",
        },
      ],
    },
    {
      id: "layout",
      title: "Finding your way around",
      summary: "What the Projects, Files and Code panels are for",
      blocks: [
        {
          type: "terms",
          items: [
            {
              term: "Projects",
              text: "On the left. The project list on top; pick one to see its **Current version**, its versions, **Save history** and **Unsaved changes** below.",
            },
            { term: "Files", text: "In the middle. The project's files: create, rename, drag and delete them, and manage hidden files." },
            { term: "Code", text: "On the right. Shows the chosen file, which you can also edit and save." },
          ],
        },
        { type: "heading", text: "Arrange it your way" },
        {
          type: "list",
          items: [
            "The layout button next to the Projects title switches between **side by side** and **stacked** panels.",
            "Drag the lines between panels to resize them; **double-click** a line to restore its size.",
            "Click a panel's title to fold it away and make room for the others.",
            "Top right: the **User guide** (this) and **Settings**.",
          ],
        },
      ],
    },
    {
      id: "saving",
      title: "Saving and going back",
      summary: "Save records, read their details, return to any of them",
      blocks: [
        { type: "heading", text: "Save" },
        {
          type: "paragraph",
          text: "**Unsaved changes** lists every change, marked **Edit**, **Add**, **Del** or **Renamed**. Click **Save version…** and write a title and details. The title may stay empty; a default one is used.",
        },
        { type: "heading", text: "Go back to a save" },
        {
          type: "steps",
          items: [
            "Click the record you want in **Save history**.",
            "Confirm, and the project's files become what that record holds. Later records stay in the list, so you can come back to them any time.",
            "Changes you had not saved are kept as **Unsaved changes** at the top of the list; one click brings them back.",
          ],
        },
        { type: "heading", text: "Tidy the history" },
        {
          type: "list",
          items: [
            "The arrow on a record shows its details.",
            "**Right-click** a record to **Edit title and details** or **Delete this record**. Deleting only takes it out of the list; the other records keep their content.",
          ],
        },
        {
          type: "tip",
          text: "Say what you did in the title, such as \"Fix login button doing nothing\"; you will thank yourself when looking for it later.",
        },
      ],
    },
    {
      id: "versions",
      title: "Versions",
      summary: "Try an idea without risking what you have",
      blocks: [
        {
          type: "paragraph",
          text: "Versions are parallel lines of the same project. Planning a big redesign? Start a version called \"new look\" and work there; merge it back when it works out, or simply switch back if it does not. The original stays untouched either way.",
        },
        {
          type: "list",
          items: [
            "**New**: click **New** in the version list's toolbar and give it a name; you can switch to it right away.",
            "**Switch**: click a version in the list and the project's files become its latest state.",
            "**Merge**: brings another version's work into the current one. If both changed the same lines, you are told about the conflict and settle it by hand.",
            "**Delete**: removes a version you no longer need.",
          ],
        },
        { type: "heading", text: "Switching with unsaved changes?" },
        {
          type: "terms",
          items: [
            { term: "Keep & Switch", text: "Recommended. The changes wait in the current version until you come back." },
            { term: "Save & Switch", text: "Saves them as a new record of the current version first." },
            { term: "Discard & Switch", text: "Drops them. A safety copy is kept quietly, though it is not shown in the app." },
          ],
        },
      ],
    },
    {
      id: "files",
      title: "Files and editing",
      summary: "Manage and edit files right in the app",
      blocks: [
        { type: "heading", text: "Manage files" },
        {
          type: "list",
          items: [
            "The top of the Files panel has **New file**, **New folder** and **Refresh**.",
            "**Right-click** a file or folder to create, **Rename**, **Stop saving** or **Delete** it.",
            "Drag a file onto another folder to move it.",
          ],
        },
        { type: "warning", text: "**Delete** here removes the file from disk and cannot be undone. Save first, and you can still get it back from the save history." },
        { type: "heading", text: "View and edit" },
        {
          type: "steps",
          items: [
            "Click a file in the tree to see it in the Code panel. It opens **Read-only** so nothing changes by accident.",
            "Click **Read-only** in the toolbar to switch to **Editing**.",
            "Press `Ctrl+S` or click **Save** to write it to disk. The change then shows up in **Unsaved changes**, ready to be saved as a version.",
          ],
        },
        {
          type: "tip",
          text: "If the file changed elsewhere after you opened it (going back to a record, for example), you are asked before it is overwritten.",
        },
      ],
    },
    {
      id: "hiding",
      title: "Hidden files",
      summary: "Keep dependencies and build output out of your snapshots",
      blocks: [
        {
          type: "paragraph",
          text: "Some files do not belong in snapshots: dependencies such as `node_modules/` that can be reinstalled, build output, logs. Hidden files stay out of Unsaved changes and take no room in your snapshots.",
        },
        {
          type: "steps",
          items: [
            "Click **Hidden files** at the top of the Files panel.",
            "Type a rule and click **Add**: `node_modules/` (a whole folder), `*.log` (every log file), `/build/` (the build folder at the project root).",
            "Or right-click a file or folder in the tree and choose **Stop saving**.",
          ],
        },
        {
          type: "list",
          items: [
            "Rules are written to the project's `.gitignore`, so they are saved and uploaded with it and work on other computers too.",
            "Rules under **Hidden by Project Hub** apply on this computer only and need no attention.",
            "Files saved before they were hidden stop being saved (they stay on disk); it takes effect with the next save.",
          ],
        },
      ],
    },
    {
      id: "cloud",
      title: "Cloud backup (GitHub)",
      summary: "Upload to GitHub and carry on from another computer",
      blocks: [
        {
          type: "steps",
          items: [
            "Open **Settings → GitHub account** and click **Sign in with browser**. Paste the code shown into the GitHub page that opens and click \"Authorize\".",
            "Back in the project, click **Upload** next to the current version. The first time, choose a **New repository** (private if you like) or an **Existing repository**.",
            "From then on, one click on **Upload** after saving keeps the cloud up to date; the button tells you how many records are waiting.",
          ],
        },
        { type: "action", action: "openSettings", label: "Open Settings to sign in" },
        { type: "heading", text: "Continue on another computer" },
        {
          type: "paragraph",
          text: "Add the same project there and click **Download** to fetch new records. If both computers saved something new, you choose: **Use the cloud's** keeps this computer's extra records as a new version, so nothing is lost; **Use this computer's** overwrites the cloud with yours.",
        },
        {
          type: "tip",
          text: "Only saved records are uploaded; unsaved changes always stay on your computer.",
        },
      ],
    },
    {
      id: "agents",
      title: "Let an agent save for you (MCP)",
      summary: "Connect Claude Code and other coding agents to describe and save your work",
      blocks: [
        {
          type: "paragraph",
          text: "Project Hub runs an MCP server that only programs on this computer can reach. Once connected, a coding agent can see what changed, write the save notes, save a version and manage hidden files.",
        },
        {
          type: "steps",
          items: [
            "Open **Settings → Agent integration (MCP)** and check that it says **Running**.",
            "Under **Connection setup**, copy the **Claude Code · every folder (recommended)** command and run it once in a terminal. For other agents (Cursor, Claude Desktop) copy their configuration instead.",
            "Restart the agent and ask: \"Look at what I changed and save a version.\"",
          ],
        },
        { type: "action", action: "openSettings", label: "Open Settings to connect an agent" },
        {
          type: "list",
          items: [
            "**Tools** lists everything the agent can use; open one to see its parameters, an example call and what it returns.",
            "Tools marked **Read only** have **Try it**, which shows exactly what the agent would get.",
            "After updating Project Hub, reconnect the agent (restarting it is enough) to pick up new tools.",
          ],
        },
      ],
    },
    {
      id: "transfer",
      title: "Moving to another computer",
      summary: "Pack projects and all their snapshots into one file",
      blocks: [
        {
          type: "steps",
          items: [
            "On the old computer, open **Settings → Move to another computer** and click **Export** to get a file.",
            "Copy it to the new computer, click **Import** in the same place, choose a folder for each project and confirm.",
          ],
        },
        {
          type: "paragraph",
          text: "Every version, save record and unsaved change comes along, and so do local settings files such as `.env`. Folders that can be rebuilt, such as `node_modules` and build output, are left out; reinstall them on the new computer.",
        },
      ],
    },
    {
      id: "settings",
      title: "Settings at a glance",
      summary: "What each setting is for",
      blocks: [
        {
          type: "terms",
          items: [
            { term: "Language and theme", text: "中文 or English, dark or light." },
            { term: "About & updates", text: "Check for and install new versions, or let it check by itself." },
            { term: "Startup", text: "Start with your computer, minimized to the tray." },
            { term: "GitHub account", text: "Sign in to upload to the cloud." },
            { term: "Saved by", text: "The name and email recorded with each save; empty means your GitHub account's." },
            { term: "Code colors", text: "Colors of the Code panel: follow the app theme or pick a VS Code-style theme." },
            { term: "Agent integration (MCP)", text: "Turn the local MCP server on or off, change its port, copy connection setup." },
            { term: "Logs", text: "What happened to versions and to the app itself; handy when something goes wrong." },
            { term: "Move to another computer", text: "Export and import projects." },
          ],
        },
      ],
    },
    {
      id: "shortcuts",
      title: "Keyboard shortcuts",
      summary: "Common actions, one step faster",
      blocks: [
        {
          type: "keys",
          items: [
            { keys: ["Ctrl", "S"], text: "Save the file being edited" },
            { keys: ["Ctrl", "Z"], text: "Undo an edit" },
            { keys: ["Ctrl", "Y"], text: "Redo an edit" },
            { keys: ["Ctrl", "Enter"], text: "Save from the save dialog" },
            { keys: ["Esc"], text: "Close a dialog, menu or Settings" },
          ],
        },
        { type: "tip", text: "On macOS use `⌘` instead of `Ctrl`." },
      ],
    },
    {
      id: "faq",
      title: "Questions and answers",
      summary: "Look here first when something seems off",
      blocks: [
        {
          type: "qa",
          question: "A project says \"Folder not found\". What now?",
          answer: "The folder was most likely renamed or moved. Click **Locate folder** and pick its new place; every snapshot comes back.",
        },
        {
          type: "qa",
          question: "Does removing a project from the list delete my files?",
          answer: "No. Removing only stops Project Hub from showing it; the folder and its snapshots stay as they are, and you can add it again any time.",
        },
        {
          type: "qa",
          question: "It says it cannot run Git?",
          answer: "Project Hub uses Git on your computer to keep snapshots. Install Git (git-scm.com) and open the app again.",
        },
        {
          type: "qa",
          question: "Is the app still running after I close the window?",
          answer: "Yes. Project Hub keeps running in the tray so agents can always reach it. To quit completely, right-click the tray icon and choose 退出 (Quit).",
        },
        {
          type: "qa",
          question: "Uploading says the cloud has different records?",
          answer: "The cloud has records this computer does not: another computer uploaded them, or you went back to an older record. To keep both, click **Download** first; if this computer is right, choose **Overwrite cloud**.",
        },
        {
          type: "qa",
          question: "A file keeps showing up in unsaved changes but I never want it saved?",
          answer: "Hide it: right-click it in the file tree and choose **Stop saving**.",
        },
      ],
    },
  ],
};

export default guide;
