//SPDX-License-Identifier: Unlicense
pragma solidity 0.8.7;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Burnable.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Pausable.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Enumerable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/INFTFactory.sol";

contract BridgedNFT is
    Ownable,
    ERC721Enumerable,
    ERC721Burnable,
    ERC721Pausable
{
    /// @notice near account id
    string public nearAccount;

    /// @notice the bridge factory address.
    address public nftFactory;

    string public nftName;
    string public nftSymbol;

    /// @notice Withdraw event.
    event Withdraw(
        address tokenAddress,
        address sender,
        string tokenAccountId,
        uint256 tokenId,
        string recipient
    );

    constructor(
        string memory _nearAccount,
        address _nftFactory,
        address _owner,
        string memory _name,
        string memory _symbol
    ) ERC721(_name, _symbol) Ownable() {
        nearAccount = _nearAccount;
        nftFactory = _nftFactory;
        nftName = _name;
        nftSymbol = _symbol;
        _transferOwnership(_owner);
    }

    function setBaseURI(string memory baseURI_) external onlyOwner() {
        _baseURIextended = baseURI_;
    }
    
    function _setTokenURI(uint256 tokenId, string memory _tokenURI) internal virtual {
        require(_exists(tokenId), "ERC721Metadata: URI set of nonexistent token");
        _tokenURIs[tokenId] = _tokenURI;
    }
    
    function _baseURI() internal view virtual override returns (string memory) {
        return _baseURIextended;
    }
    
    function tokenURI(uint256 tokenId) public view virtual override returns (string memory) {
        require(_exists(tokenId), "ERC721Metadata: URI query for nonexistent token");

        string memory _tokenURI = _tokenURIs[tokenId];
        string memory base = _baseURI();
        
        // If there is no base URI, return the token URI.
        if (bytes(base).length == 0) {
            return _tokenURI;
        }
        // If both are set, concatenate the baseURI and tokenURI (via abi.encodePacked).
        if (bytes(_tokenURI).length > 0) {
            return string(abi.encodePacked(base, _tokenURI));
        }
        // If there is a baseURI but no tokenURI, concatenate the tokenID to the baseURI.
        return string(abi.encodePacked(base, tokenId.toString()));
    }
    /// @notice This function should only be called from the nft factory, it allows to mint a
    /// new nft token.
    /// @param _tokenId nft token id.
    /// @param _recipient owner of the nft.
    function mintNFT(uint256 _tokenId, address _recipient) external {
        require(msg.sender == nftFactory, "Caller is not the nft factory");
        _safeMint(_recipient, _tokenId);
        string _uri = tokenURI(_tokenId);
        _setTokenURI(_tokenId, _uri);
    }

    /// @notice This function allows to start the process of unlock the token from near side,
    /// by burning the nft token.
    /// @param _tokenId nft token id.
    function withdrawNFT(uint256 _tokenId, string memory _recipientNearAccount)
        external
    {
        require(
            !INFTFactory(nftFactory).pauseBridgedWithdraw(),
            "Withdrawal is disabled"
        );
        _burn(_tokenId);

        // emit Withdraw event
        emit Withdraw(
            address(this),
            msg.sender,
            nearAccount,
            _tokenId,
            _recipientNearAccount
        );
    }

    function _beforeTokenTransfer(
        address from,
        address to,
        uint256 tokenId
    ) internal virtual override(ERC721, ERC721Enumerable, ERC721Pausable) {
        super._beforeTokenTransfer(from, to, tokenId);
    }

    function supportsInterface(bytes4 interfaceId)
        public
        view
        virtual
        override(ERC721, ERC721Enumerable)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }
}
