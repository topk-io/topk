class CollectionAlreadyExistsError(Exception):
  """
  Raised when creating a collection with a name that already exists.
  """


class CollectionNotFoundError(Exception):
  """
  Raised when a collection is not found.
  """


class CollectionValidationError(Exception):
  """
  Raised when a collection name or schema is invalid.
  """


class DocumentValidationError(Exception):
  """
  Raised when a document is invalid.
  """


class InvalidArgumentError(Exception):
  """
  Raised when an invalid argument is provided.
  """


class PermissionDeniedError(Exception):
  """
  Raised when a permission is denied.
  """


class QueryLsnTimeoutError(Exception):
  """
  Raised when a query LSN timeout occurs.
  """


class QuotaExceededError(Exception):
  """
  Raised when a quota is exceeded.
  """


class RequestTooLargeError(Exception):
  """
  Raised when a request is too large.
  """


class SchemaValidationError(Exception):
  """
  Raised when a schema is invalid.
  """


class SlowDownError(Exception):
  """
  Raised when a slow down occurs.
  """
