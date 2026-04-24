## App-wide
app-title = Bitwarden [Next]
about-window-title = About Bitwarden

## Login — unlock
login-unlock-title = Your vault is locked
login-unlock-password-placeholder = Master password (required)
login-unlock-pin-placeholder = PIN (required)
login-unlock-button = Unlock
login-unlock-or = or
login-unlock-biometrics-button = Unlock with biometrics
login-unlock-pin-button = Unlock with PIN
login-unlock-master-password-button = Unlock with master password
login-log-out = Log out

## Login — email entry
login-email-title = Log in to Bitwarden
login-email-placeholder = Email address (required)
login-email-remember = Remember email
login-email-continue = Continue
login-email-or = Or
login-email-sso = Use single sign-on
login-email-new-prompt = New to Bitwarden?
login-email-create-account = Create account

## Login — password entry
login-password-title = Welcome back
login-password-placeholder = Master password (required)
login-password-get-hint = Get master password hint
login-password-submit = Log in with master password
login-password-back = Back

## Login — server selector
login-server-accessing = Accessing { $server }
login-server-accessing-label = Accessing:
login-server-self-hosted = Self-hosted

## Login — toasts
login-toast-unlock-failed-title = Unlock failed
login-toast-unlock-failed-body = Check your master password and try again.
login-toast-login-failed-title = Login failed
login-toast-login-failed-body = Check your email and password and try again.
login-toast-pin-unsupported = PIN unlock is not yet supported
login-toast-biometrics-unsupported = Biometric unlock is not yet supported

## About dialog
about-version-label = Version
about-sdk-version-label = SDK version
about-os-label = OS
about-architecture-label = Architecture
about-copy-button = Copy
about-close-button = Close

## Sidebar — sections and filters
sidebar-section-vault = Vault
sidebar-section-send = Send
sidebar-filter-my-vault = My Vault
sidebar-filter-favorites = Favorites
sidebar-filter-logins = Logins
sidebar-filter-cards = Cards
sidebar-filter-identities = Identities
sidebar-filter-notes = Notes
sidebar-filter-ssh-keys = SSH keys
sidebar-filter-archive = Archive
sidebar-filter-trash = Trash
sidebar-filter-text-send = Text
sidebar-filter-file-send = File
sidebar-item-generator = Generator
sidebar-item-import = Import
sidebar-item-export = Export

## Vault list
vault-title = Vault
vault-new-button = New
vault-search-placeholder = Search
vault-column-name = Name
vault-column-options = Options
vault-toast-item-saved = Item saved
vault-toast-save-failed-title = Save failed
vault-toast-save-failed-body = Couldn't save the item. Try again.
vault-toast-decrypt-failed-title = Decrypt failed
vault-toast-decrypt-failed-body = Couldn't load the item. Try again.
vault-toast-copied-username = Username copied
vault-toast-copied-password = Password copied
vault-toast-copied-website = Website copied
vault-toast-copied-totp = Verification code copied
vault-toast-copied-field = Field copied
vault-toast-copied-private-key = Private key copied
vault-toast-copied-public-key = Public key copied
vault-toast-copied-fingerprint = Fingerprint copied
vault-toast-item-deleted = Item moved to trash
vault-toast-delete-failed-title = Delete failed
vault-toast-delete-failed-body = Couldn't delete the item. Try again.
vault-delete-modal-title = Delete item?
vault-delete-modal-body = "{ $name }" will be moved to the trash.
vault-delete-modal-cancel = Cancel
vault-delete-modal-confirm = Delete

## Account switcher
account-switcher-other-accounts = Other Bitwarden accounts
account-switcher-options = Options
account-switcher-lock-now = Lock now
account-switcher-log-out = Log out
account-switcher-lock-all = Lock all accounts
account-switcher-settings = Settings
account-switcher-add = Add account

## Detail pane — headers per cipher type
detail-header-login = View login
detail-header-card = View card
detail-header-identity = View identity
detail-header-note = View note
detail-header-ssh-key = View SSH key

## Detail pane — section labels
detail-section-item-details = Item details
detail-section-login-credentials = Login credentials
detail-section-autofill-options = Autofill options
detail-section-card-details = Card details
detail-section-personal-details = Personal details
detail-section-note = Note
detail-section-ssh-key = SSH key

## Detail pane — fields
detail-field-name = Name
detail-field-notes = Notes
detail-field-username = Username
detail-field-password = Password
detail-field-totp = Verification code (TOTP)
detail-totp-invalid = Invalid TOTP seed
detail-field-website = Website
detail-field-cardholder-name = Cardholder name
detail-field-brand = Brand
detail-field-number = Number
detail-field-expiration = Expiration
detail-field-security-code = Security code
detail-field-email = Email
detail-field-phone = Phone
detail-field-company = Company
detail-field-address = Address
detail-field-city-region = City / region
detail-field-public-key = Public key
detail-field-private-key = Private key
detail-field-fingerprint = Fingerprint
detail-empty-credentials = No credentials
detail-empty-card = No card details
detail-empty-identity = No identity details
detail-section-custom-fields = Custom fields
detail-field-passkey = Passkey
detail-field-passkey-created = Created { $date }
detail-field-boolean-true = Yes
detail-field-boolean-false = No
detail-edit-button = Edit

## Cipher form — header titles
form-title-new-item = New item
form-title-edit-login = Edit login
form-title-edit-card = Edit card
form-title-edit-identity = Edit identity
form-title-edit-note = Edit note
form-title-edit-ssh-key = Edit SSH key

## Cipher form — buttons
form-save = Save
form-saving = Saving…
form-cancel = Cancel

## Cipher form — sections
form-section-item-details = Item details
form-section-login-credentials = Login credentials
form-section-autofill-options = Autofill options
form-section-card-details = Card details
form-section-personal-details = Personal details
form-section-identification = Identification
form-section-contact-info = Contact info
form-section-address = Address
form-section-ssh-key = SSH key
form-section-additional-options = Additional options
form-section-custom-fields = Custom fields

## Cipher form — item details
form-name = Name (required)
form-favorite = Favorite
form-reprompt = Master password re-prompt
form-notes = Notes
form-folder = Folder
form-folder-none = No folder
form-organization = Organization
form-organization-personal = Personal (me)
form-collections = Collections
form-collections-none = No collections
form-collections-selected = { $count } selected
form-collections-empty-in-org = No collections in this org

## Cipher form — login
form-username = Username
form-password = Password
form-totp = Authenticator key (TOTP)

## Cipher form — websites
form-uri = Website (URI)
form-uri-empty = No websites yet
form-add-website = Add website

## Cipher form — card
form-card-cardholder = Cardholder name
form-card-brand = Brand
form-card-number = Number
form-card-exp-month = Expiration month
form-card-exp-year = Expiration year
form-card-code = Security code
form-card-brand-placeholder = -- Select --
form-card-month-placeholder = -- Month --

## Cipher form — identity
form-identity-title = Title
form-identity-title-placeholder = -- Title --
# Identity title labels. Same convention as card brands — the stored value
# is the canonical English string; these keys only localize the display.
form-identity-title-mr = Mr
form-identity-title-mrs = Mrs
form-identity-title-ms = Ms
form-identity-title-mx = Mx
form-identity-title-dr = Dr
form-identity-first-name = First name
form-identity-middle-name = Middle name
form-identity-last-name = Last name
form-identity-username = Username
form-identity-company = Company
form-identity-ssn = Social Security number
form-identity-passport = Passport number
form-identity-license = License number
form-identity-email = Email
form-identity-phone = Phone
form-identity-address1 = Address line 1
form-identity-address2 = Address line 2
form-identity-address3 = Address line 3
form-identity-city = City / town
form-identity-state = State / province
form-identity-postal = Zip / postal code
form-identity-country = Country

## Cipher form — SSH key
form-ssh-public-key = Public key
form-ssh-private-key = Private key
form-ssh-fingerprint = Fingerprint

## Cipher form — custom fields
form-custom-field-type = Type
form-custom-field-name = Name
form-custom-field-value = Value
form-custom-field-type-text = Text
form-custom-field-type-hidden = Hidden
form-custom-field-type-boolean = Boolean
form-custom-field-type-linked = Linked
form-custom-field-enabled = Enabled
form-custom-field-empty = No custom fields yet
form-add-custom-field = Add custom field
form-custom-field-linked-unsupported = Linked fields not yet supported

## Toast — default titles
toast-default-info = Info
toast-default-success = Success
toast-default-warning = Warning
toast-default-error = Error

## Toast — shared messages
toast-required-fields = Please fill in the required fields.

## Menu bar — top-level
menu-file = File
menu-edit = Edit
menu-view = View
menu-account = Account
menu-window = Window
menu-help = Help

## Menu — File
menu-file-new-login = New login
menu-file-new-item = New item
menu-file-new-item-login = Login
menu-file-new-item-card = Card
menu-file-new-item-identity = Identity
menu-file-new-item-secure-note = Secure note
menu-file-new-item-ssh-key = SSH key
menu-file-new-folder = New folder
menu-file-sync-now = Sync now
menu-file-import = Import
menu-file-export = Export
menu-file-settings = Settings
menu-file-lock-vault = Lock vault
menu-file-lock-all-vaults = Lock all vaults
menu-file-log-out = Log out
menu-file-quit = Quit Bitwarden

## Menu — Edit
menu-edit-undo = Undo
menu-edit-redo = Redo
menu-edit-cut = Cut
menu-edit-copy = Copy
menu-edit-paste = Paste
menu-edit-select-all = Select all
menu-edit-copy-username = Copy username
menu-edit-copy-password = Copy password
menu-edit-copy-totp = Copy verification code (TOTP)

## Menu — View
menu-view-search = Search vault
menu-view-generator = Generator
menu-view-generator-history = Generator history
menu-view-zoom-in = Zoom in
menu-view-zoom-out = Zoom out
menu-view-reset-zoom = Reset zoom
menu-view-toggle-fullscreen = Toggle full screen
menu-view-reload = Reload

## Menu — Account
menu-account-premium = Premium membership
menu-account-change-password = Change master password
menu-account-two-step = Two-step login
menu-account-fingerprint = Fingerprint phrase
menu-account-delete = Delete account

## Menu — Window
menu-window-minimize = Minimize
menu-window-hide-to-tray = Hide to tray
menu-window-always-on-top = Always on top
menu-window-close = Close

## Menu — Help
menu-help-feedback = Help & feedback
menu-help-bug = File a bug report
menu-help-legal = Legal
menu-help-legal-tos = Terms of service
menu-help-legal-privacy = Privacy policy
menu-help-follow = Follow us
menu-help-web-vault = Go to web vault
menu-help-mobile-app = Get mobile app
menu-help-browser-extension = Get browser extension
menu-help-troubleshooting = Troubleshooting
menu-help-troubleshooting-gpu = Toggle hardware acceleration
menu-help-about = About Bitwarden

## Tray
tray-show-hide = Show / Hide
tray-lock-vault = Lock vault
tray-exit = Exit

# Native names shown in the Appearance tab's language picker. DO NOT
# TRANSLATE — each value should always be the language's own endonym
# (English, Español, 日本語, …) regardless of the active locale. Defined
# only here so translators of other .ftl files never accidentally rewrite
# them; the runtime looks `language-name-<tag>` up against this file directly.
language-name-en = English
language-name-es = Español

## Settings modal
settings-title = Settings
settings-tab-security = Security
settings-tab-integrations = Integrations
settings-tab-autotype = Autotype and copy
settings-tab-appearance = Appearance
settings-tab-advanced = Advanced

settings-toast-not-supported = This setting isn't supported yet.

# Security tab
settings-security-access-options = Access options
settings-security-open-at-login = Open Bitwarden at device login
settings-security-unlock-pin = Unlock with PIN
settings-security-unlock-touch = Unlock with Touch ID
settings-security-session-timeout = Session timeout
settings-security-lock-after = Lock after
settings-security-logout-after = Log out after

# Shared duration strings used by all the time-based dropdowns (lock after,
# log out after, clear clipboard after). Plurals come from Fluent selectors
# so every language can pick the right variant for the supplied number.
settings-duration-seconds = { $n ->
    [one] { $n } second
   *[other] { $n } seconds
  }
settings-duration-minutes = { $n ->
    [one] { $n } minute
   *[other] { $n } minutes
  }
settings-duration-hours = { $n ->
    [one] { $n } hour
   *[other] { $n } hours
  }
settings-duration-never = Never

# Integrations tab
settings-integrations-browser = Browser integration
settings-integrations-browser-enable = Enable browser integration
settings-integrations-browser-fingerprint = Require verification fingerprint
settings-integrations-ssh = SSH agent
settings-integrations-ssh-enable = Enable SSH agent
settings-integrations-ssh-prompt = Prompt behavior
settings-ssh-prompt-always = Always
settings-ssh-prompt-never = Never
settings-ssh-prompt-remember = Remember until lock
settings-integrations-other = Other
settings-integrations-duckduckgo = Enable DuckDuckGo browser integration

# Autotype and copy tab
settings-autotype-heading = Autotype
settings-autotype-enable = Enable autotype
settings-clipboard-heading = Clipboard
settings-clipboard-clear-after = Clear clipboard after
settings-clipboard-minimize-on-copy = Minimize on copy

# Appearance tab
settings-appearance-theme-heading = Theme
settings-appearance-theme = Theme
settings-appearance-theme-system = System
settings-appearance-theme-light = Light
settings-appearance-theme-dark = Dark
settings-appearance-language = Language
settings-appearance-language-system = System
settings-appearance-display-heading = Display
settings-appearance-show-favicons = Show icons for URLs

# Advanced tab
settings-advanced-tray = Tray
settings-advanced-tray-enable = Show tray icon
settings-advanced-minimize-to-tray = Minimize to tray
settings-advanced-close-to-tray = Close to tray
settings-advanced-start-to-tray = Start to tray
settings-advanced-platform = Platform
settings-advanced-always-show-dock = Always show dock icon
settings-advanced-hardware-acceleration = Enable hardware acceleration
settings-advanced-allow-screenshots = Allow screenshots

## Send list
send-title = Send
send-new-button = New
send-search-placeholder = Search Sends
send-column-name = Name
send-column-deletion = Deletion date
send-empty-body = You haven't created any Sends yet.
send-toast-item-saved = Send saved
send-toast-item-deleted = Send deleted
send-toast-copied-link = Send link copied
send-toast-copied-password = Password copied
send-toast-load-failed-title = Load failed
send-toast-load-failed-body = Couldn't load the Send. Try again.
send-toast-save-failed-title = Save failed
send-toast-save-failed-body = Couldn't save the Send. Try again.
send-toast-delete-failed-title = Delete failed
send-toast-delete-failed-body = Couldn't delete the Send. Try again.

## Send form
send-form-title-new-text = Create Text Send
send-form-title-new-file = Create File Send
send-form-title-edit-text = Edit Text Send
send-form-title-edit-file = Edit File Send
send-form-details-heading = Send details
send-form-additional-heading = Additional options
send-form-name = Name (required)
send-form-text = Text to share (required)
send-form-hide-text = Hide text by default
send-form-file-choose = Choose file
send-form-file-choose-placeholder = No file selected
send-form-file-name = File name
send-form-file-size = Size
send-form-deletion-date = Deletion date
send-form-deletion-hint = The Send will be permanently deleted on {$date}.
send-form-who-can-view = Who can view
send-form-access-link = Anyone with the link
send-form-access-people = Specific people
send-form-access-password = Anyone with a password set by you
send-form-password = Password (required)
send-form-password-hint = Individuals will need to enter the password to view this Send.
send-form-emails = Emails (required)
send-form-send-link = Send link
send-form-limit-views = Limit views
send-form-limit-views-hint = No one can view this Send after the limit is reached.
send-form-views-left = No one can view this Send after the limit is reached. {$count} views left.
send-form-hide-email = Hide your email address from viewers.
send-form-private-note = Private note
send-form-save = Save
send-form-cancel = Cancel

send-form-preset-1h = 1 hour
send-form-preset-1d = 1 day
send-form-preset-2d = 2 days
send-form-preset-3d = 3 days
send-form-preset-7d = 7 days
send-form-preset-14d = 14 days
send-form-preset-30d = 30 days

send-delete-modal-title = Delete Send
send-delete-modal-body = Are you sure you want to delete "{$name}"?
send-delete-modal-cancel = Cancel
send-delete-modal-confirm = Delete

## Generator
generator-title = Generator
generator-tab-password = Password
generator-tab-passphrase = Passphrase
generator-tab-username = Username
generator-options = Options

# Password tab
generator-length = Length
generator-length-hint = Value must be between 5 and 128.
generator-include = Include
generator-include-uppercase = A-Z
generator-include-lowercase = a-z
generator-include-numbers = 0-9
generator-include-special = !@#$%^&*
generator-min-number = Minimum numbers
generator-min-special = Minimum special
generator-avoid-ambiguous = Avoid ambiguous characters

# Passphrase tab
generator-num-words = Number of words
generator-num-words-hint = Value must be between 3 and 20. Use 6 words or more to generate a strong passphrase.
generator-word-separator = Word separator
generator-passphrase-capitalize = Capitalize
generator-passphrase-include-number = Include number

# Username tab
generator-username-type = Type
generator-username-kind-word = Random word
generator-username-kind-subaddress = Plus addressed email
generator-username-kind-catchall = Catch-all email
generator-username-capitalize = Capitalize
generator-username-include-number = Include number
generator-username-email = Email
generator-username-domain = Domain

# History
generator-history-open = Generator history
generator-history-title = Generator history
generator-history-heading = Recent
generator-history-empty = No recent values.
generator-history-clear = Clear history
generator-history-just-now = just now
generator-history-minutes-ago = {$count}m ago
generator-history-hours-ago = {$count}h ago
generator-history-days-ago = {$count}d ago

# Toasts
generator-toast-copied = Copied
generator-toast-failed = Couldn't generate
