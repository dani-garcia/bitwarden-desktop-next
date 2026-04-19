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
vault-toast-item-deleted = Item moved to trash
vault-toast-delete-failed-title = Delete failed
vault-toast-delete-failed-body = Couldn't delete the item. Try again.
vault-delete-modal-title = Delete item?
vault-delete-modal-body = "{ $name }" will be moved to the trash.
vault-delete-modal-cancel = Cancel
vault-delete-modal-confirm = Delete

## Account switcher
account-switcher-locked-suffix = (locked)
account-switcher-add = + Add account

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
form-organization = Organization
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
